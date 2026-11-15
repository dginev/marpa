//! libmarpa-asf-sys builds `marpa_obs.c` with a per-thread cache of freed
//! default-size (16 KiB) obstack chunks (`libmarpa-sys/c/marpa_obs.c`), held by a
//! pthread key whose destructor frees it at thread exit. Recycled chunks hold stale
//! bytes, so every parse must equal the first; the cache's test-only counters
//! (`chunk-cache-stats`) prove the paths ran: hits, a full cache (the 64-chunk cap),
//! and chunks larger than 16 KiB, which must never be served from the cache.
extern crate marpa_asf as marpa;

use marpa::grammar::Grammar;
use marpa::lexer::byte_scanner::*;
use marpa::parser::*;
use marpa::result::Result;
use marpa::stack::*;
use marpa::thin::Tree;
use marpa::tree_builder::*;

use std::io::Cursor;

/// The cache exists on every target but Windows and WebAssembly (`marpa_obs.c`).
const CACHED: bool = cfg!(not(any(windows, target_family = "wasm")));
const CHUNK_CACHE_MAX: u64 = 64;

/// Parse `input` and keep everything alive: the value, and the parser and tree that own the
/// recognizer's, bocage's and order's obstacks.
fn parse_held(input: &str) -> Result<(String, Parser, Tree)> {
    let mut g = Grammar::new()?;
    let ws_char = g.string_set(None, "\t\n\r ")?;
    let ws = g.star(None, ws_char)?;
    let sep = g.literal_string(None, "::=")?;
    let term = g.literal_string(None, ";")?;
    let dquote = g.literal_string(None, "\"")?;
    let not_dquote = g.inverse_string_set(None, "\"")?;
    let str_chars_star = g.star(None, not_dquote)?;
    let string = g.rule(None, &[dquote, str_chars_star, dquote])?;
    let lower = g.char_range(None, 'a', 'z')?;
    let ident = g.plus(None, lower)?;
    let rule = g.rule(None, &[ident, ws, sep, ws, string, ws, term])?;
    let rules = g.sequence(None, rule, ws_char, false, false)?;
    g.set_start(rules)?;

    let mut b = TreeBuilder::new();
    for r in [rules, rule].iter().map(|x| x.rule()) {
        b.rule(r);
    }
    for t in [ident, string, sep, term].iter().map(|x| x.rule()) {
        b.token(t);
    }
    for d in [ws_char].iter().map(|x| x.rule()) {
        b.discard(d);
    }
    let mut p = Parser::with_grammar(g.unwrap());
    let mut t = p.run_recognizer(ByteScanner::new(Cursor::new(input.to_owned())))?;
    let v = t.next().unwrap();
    Ok((format!("{}", proc_value(b, v)), p, t))
}

fn parse_once(input: &str) -> Result<String> {
    parse_held(input).map(|(value, _, _)| value)
}

/// A grammar of `n` keywords: its precompute arrays, sized by the grammar, and the
/// recognizer's per-symbol arrays are single obstack objects over 16 KiB, requested while
/// the thread's cache holds the earlier parses' default chunks.
fn parse_keywords(n: usize) -> Result<String> {
    let mut g = Grammar::new()?;
    let ws_char = g.string_set(None, " ")?;
    let words: Vec<String> = (0..n).map(keyword).collect();
    let mut kws = Vec::new();
    for w in &words {
        kws.push(g.literal_string(None, w.as_str())?);
    }
    let kw = g.alternative(None, &kws)?;
    let doc = g.sequence(None, kw, ws_char, false, false)?;
    g.set_start(doc)?;
    let mut b = TreeBuilder::new();
    b.rule(doc.rule());
    for k in &kws {
        b.token(k.rule());
    }
    b.discard(ws_char.rule());
    let input = words.iter().step_by(37).cloned().collect::<Vec<_>>().join(" ");
    let mut p = Parser::with_grammar(g.unwrap());
    let mut t = p.run_recognizer(ByteScanner::new(Cursor::new(input)))?;
    let v = t.next().unwrap();
    Ok(format!("{}", proc_value(b, v)))
}

/// The `i`th keyword: letters only, none a prefix of another.
fn keyword(i: usize) -> String {
    let mut w = String::from("k");
    let mut j = i;
    for _ in 0..4 {
        w.push((b'a' + (j % 26) as u8) as char);
        j /= 26;
    }
    w
}

fn rules_input(n: usize) -> String {
    let input: String = (0..n).map(|i| format!("r{} ::= \"v{i}\";\n", "abc".repeat(i % 7 + 1))).collect();
    input.replace(|c: char| c.is_ascii_digit(), "q")
}

#[test]
fn recycled_chunks_parse_identically_across_threads() {
    // 3000 rules: the bocage's and order's per-node arrays are single obstack objects
    // over 16 KiB, so the oversize path runs beside the cached one.
    let big = rules_input(3000);
    // 300 rules: a recognizer obstack spanning many default chunks.
    let small = rules_input(300);
    let expected_big = parse_once(&big).expect("first big parse");
    let expected_small = parse_once(&small).expect("first small parse");
    let expected_kw = parse_keywords(2000).expect("first keyword parse");
    let handles: Vec<_> = (0..4)
        .map(|_| {
            let (big, small, expected_kw) = (big.clone(), small.clone(), expected_kw.clone());
            std::thread::spawn(move || {
                let mut got = Vec::new();
                for _ in 0..3 {
                    // Twelve parses alive at once, then dropped together: more than
                    // CHUNK_CACHE_MAX chunks come back, so the cache fills.
                    let held: Vec<_> = (0..12).map(|_| parse_held(&small).expect("parse")).collect();
                    got.extend(held.into_iter().map(|(value, _, _)| (false, value)));
                    got.push((true, parse_once(&big).expect("big parse")));
                    got.extend((0..4).map(|_| (false, parse_once(&small).expect("parse"))));
                    assert_eq!(parse_keywords(2000).expect("keyword parse"), expected_kw);
                }
                (got, libmarpa_sys::chunk_cache_stats())
            })
        })
        .collect();
    for h in handles {
        // Each thread exits with a full cache: its key destructor frees it.
        let (got, stats) = h.join().expect("thread");
        for (is_big, value) in got {
            assert_eq!(&value, if is_big { &expected_big } else { &expected_small });
        }
        if CACHED {
            assert!(stats.hits > 100, "the cache served chunks: {stats:?}");
            assert_eq!(stats.peak, CHUNK_CACHE_MAX, "the cache filled to its cap: {stats:?}");
            assert!(stats.over_cap > 0, "a full cache freed chunks: {stats:?}");
            assert!(stats.oversize > 0, "chunks over 16 KiB were requested: {stats:?}");
            assert!(stats.count <= CHUNK_CACHE_MAX, "{stats:?}");
        } else {
            assert_eq!(stats, libmarpa_sys::ChunkCacheStats::default());
        }
    }
}

/// A parse from a TSD destructor that runs after the cache's own (the B3 class: memory used
/// at thread exit). The cache's destructor freed the thread's cache and left the key a static
/// "retired" marker, so this parse's chunks go straight to free(): it gets no fresh cache to
/// leak, and the thread exits cleanly.
#[cfg(unix)]
#[test]
fn parse_from_a_later_tsd_destructor_bypasses_the_retired_cache() {
    use std::ffi::c_void;
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static KEY: Mutex<Option<libc::pthread_key_t>> = Mutex::new(None);
    static ROUNDS: AtomicUsize = AtomicUsize::new(0);
    static LATE: Mutex<Option<(String, libmarpa_sys::ChunkCacheStats)>> = Mutex::new(None);

    extern "C" fn late(value: *mut c_void) {
        // Round one may run before or after the cache's destructor (key order is the
        // system's); asking for a second round puts this parse after it on every system.
        if ROUNDS.fetch_add(1, Ordering::SeqCst) == 0 {
            let key = KEY.lock().unwrap().expect("key");
            unsafe { libc::pthread_setspecific(key, value) };
            return;
        }
        let value = parse_once(&rules_input(300)).expect("late parse");
        *LATE.lock().unwrap() = Some((value, libmarpa_sys::chunk_cache_stats()));
    }

    let small = rules_input(300);
    let expected = parse_once(&small).expect("first parse");
    std::thread::spawn(move || {
        parse_once(&small).expect("parse");
        let mut key: libc::pthread_key_t = 0;
        assert_eq!(unsafe { libc::pthread_key_create(&mut key, Some(late)) }, 0);
        *KEY.lock().unwrap() = Some(key);
        assert_eq!(unsafe { libc::pthread_setspecific(key, std::ptr::dangling_mut::<c_void>()) }, 0);
    })
    .join()
    .expect("thread");
    let (value, stats) = LATE.lock().unwrap().take().expect("the late destructor parsed");
    assert_eq!(value, expected);
    assert_eq!(ROUNDS.load(Ordering::SeqCst), 2);
    if CACHED {
        assert_eq!(stats, libmarpa_sys::ChunkCacheStats::default(), "no cache after retirement");
    }
}
