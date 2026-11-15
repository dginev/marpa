/* Copyright 2018 Jeffrey Kegler */

/* This file is a modification of one of the versions of the GNU obstack.c
 * which was LGPL 2.1.  Here is the copyright notice from that file:
 *
 * obstack.c - subroutines used implicitly by object stack macros
 * Copyright (C) 1988, 1989, 1990, 1991, 1992, 1993, 1994, 1996, 1997, 1998,
 * 1999, 2000, 2001, 2002, 2003, 2004, 2005 Free Software Foundation, Inc.
 * This file is part of the GNU C Library.
 *
 * The GNU C Library is free software; you can redistribute it and/or
 * modify it under the terms of the GNU Lesser General Public
 * License as published by the Free Software Foundation; either
 * version 2.1 of the License, or (at your option) any later version.
 *
 * The GNU C Library is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU
 * Lesser General Public License for more details.
 *
 * You should have received a copy of the GNU Lesser General Public
 * License along with the GNU C Library; if not, write to the Free
 * Software Foundation, Inc., 51 Franklin Street, Fifth Floor,
 * Boston, MA 02110-1301, USA.  */

/* Modified 2026-11-15 for the libmarpa-asf-sys crate (dginev/marpa): a
 * 16 KiB default chunk and a per-thread cache of default-size chunks,
 * held by a pthread key (see "Chunk cache" below), poisoned for ASan and
 * valgrind while cached, with test-only counters under
 * MARPA_OBS_CHUNK_CACHE_STATS.  Otherwise identical to libmarpa 8.6.2's
 * marpa_obs.c.  */

# include "config.h"

# include "marpa.h"
# include "marpa_ami.h"
# include "marpa_obs.h"

/* This estimate of malloc's overhead is the one used in Linus Torvald's slab
 * allocator in git.  I assume that it assumes a 64-bit architecture.
 */
#define MALLOC_OVERHEAD 32
/* 16 KiB, not upstream's 4 KiB: a recognizer's or bocage's obstack grows
 * through many chunks, and objects over 4 KiB took a chunk of their own. */
#define DEFAULT_CHUNK_SIZE (16384 - MALLOC_OVERHEAD)

/* Chunk cache.  libmarpa begins and frees obstacks per earleme, per
 * recognizer, bocage, order and value, so a parse mallocs and frees many
 * default-size chunks, which miss glibc's tcache and go through its bins.
 * A freed default-size chunk is kept on a per-thread LIFO list (at most
 * CHUNK_CACHE_MAX, 1 MiB) and handed to the next obstack.  The list lives
 * in a malloc'ed struct held only by a pthread key, never in _Thread_local
 * storage: macOS frees that storage in its own key destructor, which may
 * run before this one.  Windows and WebAssembly keep plain malloc/free. */
#if !defined(_WIN32) && !defined(__wasm__)
# define MARPA_OBS_CHUNK_CACHE 1
#endif

/* Test-only counters (the crate's `chunk-cache-stats` feature).  They count
 * only what a cache exists for; the control flow is production's. */
struct marpa_asf_obs_chunk_cache_stats
{
  unsigned long long hits;      /* chunk_alloc served from the cache */
  unsigned long long kept;      /* chunk_release kept a chunk */
  unsigned long long over_cap;  /* chunk_release freed: the cache was full */
  unsigned long long oversize;  /* chunk_alloc of a non-default size, cache made */
  unsigned long long count;     /* chunks cached now */
  unsigned long long peak;      /* most chunks cached at once */
};
#ifdef MARPA_OBS_CHUNK_CACHE_STATS
# define CHUNK_STAT(stmt) stmt
#else
# define CHUNK_STAT(stmt)
#endif

#ifdef MARPA_OBS_CHUNK_CACHE
# include <pthread.h>
# include <stdlib.h>
# define CHUNK_CACHE_MAX 64

/* A cached chunk is freed memory to the program: poisoned under ASan, no-access
 * under valgrind (MARPA_OBS_VALGRIND, the crate's debug-assertion builds), so a
 * use after marpa_obs_free still reports.  Without either, nothing is emitted. */
# if defined(__SANITIZE_ADDRESS__)
#  define MARPA_OBS_ASAN 1
# elif defined(__has_feature)
#  if __has_feature(address_sanitizer)
#   define MARPA_OBS_ASAN 1
#  endif
# endif
# ifdef MARPA_OBS_ASAN
#  include <sanitizer/asan_interface.h>
# endif
# if defined(MARPA_OBS_VALGRIND) && defined(__has_include)
#  if __has_include(<valgrind/memcheck.h>)
#   include <valgrind/memcheck.h>
#   define MARPA_OBS_MEMCHECK 1
#  endif
# endif
# if defined(MARPA_OBS_ASAN) || defined(MARPA_OBS_MEMCHECK)
#  ifndef MARPA_OBS_ASAN
#   define ASAN_POISON_MEMORY_REGION(p, n) ((void) 0)
#   define ASAN_UNPOISON_MEMORY_REGION(p, n) ((void) 0)
#  endif
#  ifndef MARPA_OBS_MEMCHECK
#   define VALGRIND_MAKE_MEM_NOACCESS(p, n) ((void) 0)
#   define VALGRIND_MAKE_MEM_DEFINED(p, n) ((void) 0)
#   define VALGRIND_MAKE_MEM_UNDEFINED(p, n) ((void) 0)
#  endif
/* The chunk enters the cache. */
#  define CHUNK_HIDE(c) \
  (ASAN_POISON_MEMORY_REGION ((c), DEFAULT_CHUNK_SIZE), \
   VALGRIND_MAKE_MEM_NOACCESS ((c), DEFAULT_CHUNK_SIZE))
/* The cache reads the chunk's list link. */
#  define CHUNK_SHOW_HEADER(c) \
  (ASAN_UNPOISON_MEMORY_REGION (&(c)->header, sizeof ((c)->header)), \
   VALGRIND_MAKE_MEM_DEFINED (&(c)->header, sizeof ((c)->header)))
/* The chunk leaves the cache for an obstack: fresh, as from malloc. */
#  define CHUNK_REUSE(c) \
  (ASAN_UNPOISON_MEMORY_REGION ((c), DEFAULT_CHUNK_SIZE), \
   VALGRIND_MAKE_MEM_UNDEFINED ((c), DEFAULT_CHUNK_SIZE))
# else
#  define CHUNK_HIDE(c)
#  define CHUNK_SHOW_HEADER(c)
#  define CHUNK_REUSE(c)
# endif

struct chunk_cache
{
  struct marpa_obstack_chunk *head;     /* linked through header.prev */
  int count;
#ifdef MARPA_OBS_CHUNK_CACHE_STATS
  struct marpa_asf_obs_chunk_cache_stats stats;
#endif
};

static pthread_key_t chunk_cache_key;
static pthread_once_t chunk_cache_once = PTHREAD_ONCE_INIT;
static int chunk_cache_usable;
/* The key's value once its destructor has run: the thread is exiting, and
 * chunks released from later TSD destructors go straight to free(). */
static struct chunk_cache chunk_cache_retired;

static void
chunk_cache_destroy (void *p)
{
  struct chunk_cache *cache = p;
  if (cache != &chunk_cache_retired)
    {
      struct marpa_obstack_chunk *chunk = cache->head;
      while (chunk)
        {
          struct marpa_obstack_chunk *prev;
          CHUNK_SHOW_HEADER (chunk);
          prev = chunk->header.prev;
          my_free (chunk);
          chunk = prev;
        }
      free (cache);
    }
  /* POSIX cleared the value before this call and re-runs destructors while
   * any value is non-null, at most PTHREAD_DESTRUCTOR_ITERATIONS rounds; the
   * static marker owns nothing, so no round can leak or allocate. */
  pthread_setspecific (chunk_cache_key, &chunk_cache_retired);
}

/* The key is never deleted, so its destructor must outlive every thread:
 * dlclose of a library holding this code is unsupported, as for any Rust rlib. */
static void
chunk_cache_key_create (void)
{
  chunk_cache_usable = pthread_key_create (&chunk_cache_key, chunk_cache_destroy) == 0;
}

/* This thread's cache; with |create|, made on first use.  0 when there is
 * none: the key or the struct could not be had, or the thread is exiting. */
static struct chunk_cache *
chunk_cache_get (int create)
{
  struct chunk_cache *cache;
  if (pthread_once (&chunk_cache_once, chunk_cache_key_create) != 0 || !chunk_cache_usable)
    return 0;
  cache = pthread_getspecific (chunk_cache_key);
  if (cache == &chunk_cache_retired)
    return 0;
  if (cache || !create)
    return cache;
  cache = calloc (1, sizeof (*cache));
  if (cache && pthread_setspecific (chunk_cache_key, cache) != 0)
    {
      free (cache);
      cache = 0;
    }
  return cache;
}

static void *
chunk_alloc (size_t size)
{
  struct chunk_cache *const cache = chunk_cache_get (0);
  CHUNK_STAT (if (cache && size != DEFAULT_CHUNK_SIZE) cache->stats.oversize++);
  if (size == DEFAULT_CHUNK_SIZE && cache && cache->head)
    {
      struct marpa_obstack_chunk *const chunk = cache->head;
      CHUNK_SHOW_HEADER (chunk);
      cache->head = chunk->header.prev;
      CHUNK_REUSE (chunk);
      cache->count--;
      CHUNK_STAT ((cache->stats.hits++, cache->stats.count--));
      return chunk;
    }
  return my_malloc (size);
}

static void
chunk_release (struct marpa_obstack_chunk *chunk)
{
  struct chunk_cache *cache;
  if (chunk->header.size != DEFAULT_CHUNK_SIZE
      || !(cache = chunk_cache_get (1)))
    {
      my_free (chunk);
      return;
    }
  if (cache->count >= CHUNK_CACHE_MAX)
    {
      CHUNK_STAT (cache->stats.over_cap++);
      my_free (chunk);
      return;
    }
  chunk->header.prev = cache->head;
  CHUNK_HIDE (chunk);
  cache->head = chunk;
  cache->count++;
  CHUNK_STAT ((cache->stats.kept++, cache->stats.count++,
               cache->stats.peak = MAX (cache->stats.peak, cache->stats.count)));
}
#else
# define chunk_alloc(size) my_malloc (size)
# define chunk_release(chunk) my_free (chunk)
#endif

#ifdef MARPA_OBS_CHUNK_CACHE_STATS
/* This thread's counters; all zero where there is no cache. */
void
marpa_asf_obs_chunk_cache_stats (struct marpa_asf_obs_chunk_cache_stats *out)
{
  static const struct marpa_asf_obs_chunk_cache_stats zero;
  *out = zero;
#ifdef MARPA_OBS_CHUNK_CACHE
  {
    struct chunk_cache *const cache = chunk_cache_get (0);
    if (cache)
      *out = cache->stats;
  }
#endif
}
#endif

struct marpa_obstack *
marpa__obs_begin (size_t size)
{
  struct marpa_obstack_chunk *chunk;	/* points to new chunk */
  struct marpa_obstack *h;	/* points to new obstack */
  char *object_base;
  char *chunk_base;

  /* We ignore |size| if it specifies less than the default */
  size = MAX ((int)DEFAULT_CHUNK_SIZE, size);
  chunk_base = chunk_alloc (size);

  /* The chunk header goes at the beginning */
  chunk = (struct marpa_obstack_chunk*)chunk_base;
  chunk->header.size = size;
  chunk->header.prev = 0;

  /* Put the header of the obstack itself after the header of its first
     chunk. */
  object_base = chunk_base + sizeof(chunk->header);
  object_base = ALIGN_POINTER (chunk_base, object_base, ALIGNOF (struct marpa_obstack));
  h = (struct marpa_obstack *)object_base;
  h->chunk = chunk;
  h->minimum_chunk_size = size;

  /* Set the obstack to "idle" with the pointer just after the
     obstack header */
  object_base += sizeof(*h);
  h->next_free = h->object_base = object_base;
  return h;
}

/* Allocate a new current chunk for the obstack *H
   on the assumption that LENGTH bytes need to be added
   to the current object, or a new object of length LENGTH allocated.
   Unlike original GNU obstacks, does *NOT*
   copy any partial object from the end of the old chunk
   to the beginning of the new one.

   In this implementation, we know the next object we will
   need, and |length| and |alignment| represent that object.
   We start the new object, and return its base addess.
   */

void*
marpa__obs_newchunk (struct marpa_obstack *h, size_t length, size_t alignment)
{
  struct marpa_obstack_chunk *old_chunk = h->chunk;
  struct marpa_obstack_chunk *new_chunk;
  size_t new_size;
  const size_t contents_offset = offsetof(struct marpa_obstack_chunk, contents);
  const size_t aligned_contents_offset = ALIGN_UP(contents_offset, alignment);
  const size_t space_needed_for_alignment = aligned_contents_offset - contents_offset;

  /* Compute size for new chunk.
   * Make sure there is enough room for |length|
   * after adjusting alignment.
   */
  new_size = contents_offset + space_needed_for_alignment + length;
  new_size = MAX(new_size, h->minimum_chunk_size);

  /* Allocate and initialize the new chunk.  */
  new_chunk = chunk_alloc (new_size);
  h->chunk = new_chunk;
  new_chunk->header.prev = old_chunk;
  new_chunk->header.size = new_size;

  h->object_base =  (char *)new_chunk + contents_offset + space_needed_for_alignment;
  h->next_free = h->object_base + length;
  return h->object_base;
}

/* Free everything in H.  */
void
marpa__obs_free (struct marpa_obstack *h)
{
  struct marpa_obstack_chunk *lp;       /* below addr of any objects in this chunk */
  struct marpa_obstack_chunk *plp;      /* point to previous chunk if any */

  if (!h)
    return;                     /* Return safely if never initialized */
  lp = h->chunk;
  while (lp != 0)
    {
      plp = lp->header.prev;
      chunk_release (lp);
      lp = plp;
    }
}

/* vim: set expandtab shiftwidth=4: */
