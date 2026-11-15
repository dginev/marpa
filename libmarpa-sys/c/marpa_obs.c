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
 * per-thread cache of default-size chunks (see "Chunk cache" below).
 * Otherwise identical to libmarpa 8.6.2's marpa_obs.c.  */

# include "config.h"

# include "marpa.h"
# include "marpa_ami.h"
# include "marpa_obs.h"

/* This estimate of malloc's overhead is the one used in Linus Torvald's slab
 * allocator in git.  I assume that it assumes a 64-bit architecture.
 */
#define MALLOC_OVERHEAD 32
#define DEFAULT_CHUNK_SIZE (4096 - MALLOC_OVERHEAD)

/* Chunk cache.  libmarpa begins and frees obstacks per earleme, per
 * recognizer, bocage, order and value, so a parse mallocs and frees many
 * default-size chunks; 4 KiB requests miss glibc's tcache and go through
 * its bins.  A freed default-size chunk is kept on a per-thread LIFO list
 * (at most CHUNK_CACHE_MAX) and handed to the next obstack.  Chunks never
 * cross threads through the cache; a pthread key destructor frees the
 * list when the thread exits.  Windows builds keep plain malloc/free. */
#if !defined(_WIN32)
# define MARPA_OBS_CHUNK_CACHE 1
#endif

#ifdef MARPA_OBS_CHUNK_CACHE
# include <pthread.h>
# define CHUNK_CACHE_MAX 64

struct chunk_cache
{
  struct marpa_obstack_chunk *head;     /* linked through header.prev */
  int count;
  int registered;                       /* the key's destructor will run */
};

static _Thread_local struct chunk_cache tl_chunk_cache;
static pthread_key_t chunk_cache_key;
static pthread_once_t chunk_cache_once = PTHREAD_ONCE_INIT;
static int chunk_cache_usable;

static void
chunk_cache_drain (void *p)
{
  struct chunk_cache *cache = p;
  struct marpa_obstack_chunk *chunk = cache->head;
  while (chunk)
    {
      struct marpa_obstack_chunk *prev = chunk->header.prev;
      my_free (chunk);
      chunk = prev;
    }
  cache->head = 0;
  cache->count = 0;
  cache->registered = 0;
}

static void
chunk_cache_key_create (void)
{
  chunk_cache_usable = pthread_key_create (&chunk_cache_key, chunk_cache_drain) == 0;
}

static void *
chunk_alloc (size_t size)
{
  struct chunk_cache *const cache = &tl_chunk_cache;
  if (size == DEFAULT_CHUNK_SIZE && cache->head)
    {
      struct marpa_obstack_chunk *const chunk = cache->head;
      cache->head = chunk->header.prev;
      cache->count--;
      return chunk;
    }
  return my_malloc (size);
}

static void
chunk_release (struct marpa_obstack_chunk *chunk)
{
  struct chunk_cache *const cache = &tl_chunk_cache;
  if (chunk->header.size != DEFAULT_CHUNK_SIZE || cache->count >= CHUNK_CACHE_MAX)
    {
      my_free (chunk);
      return;
    }
  if (!cache->registered)
    {
      pthread_once (&chunk_cache_once, chunk_cache_key_create);
      if (!chunk_cache_usable || pthread_setspecific (chunk_cache_key, cache) != 0)
        {
          my_free (chunk);
          return;
        }
      cache->registered = 1;
    }
  chunk->header.prev = cache->head;
  cache->head = chunk;
  cache->count++;
}
#else
# define chunk_alloc(size) my_malloc (size)
# define chunk_release(chunk) my_free (chunk)
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
