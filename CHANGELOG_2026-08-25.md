# Changelog — 2026-08-25

- Live-verified `dalfox` and `sqlmap` after install - both worked
  correctly on the first real run. `dalfox` (`dry_run` against
  `testphp.vulnweb.com`, then a real scan against `scanme.nmap.org`
  once that test target turned out to be unreachable from this host)
  produced clean, well-structured JSON with a real 175-request scan
  and the expected "no findings" result for a static page. `sqlmap`
  printed its real legal disclaimer, correctly auto-answered its own
  confirmation prompt via `--batch`, and wrote its real per-target
  session/log/results files under `/artifacts` via `--output-dir`
  exactly as designed.

- Fixed a real, blocking `commix` bug found on its very first live
  test: the plugin failed immediately with `ImportError: cannot import
  name 'entry_point' from 'src.core.main'`. commix's own `setup.py`
  declares a console-script entry point
  (`commix = src.core.main:entry_point`), but that function doesn't
  actually exist in `src/core/main.py` at the pinned `v4.1` tag - a
  real packaging bug in the upstream project itself, not something
  visible from reading `setup.py` alone. The banner/legal disclaimer
  still printed before the crash, which was itself a clue: Python
  executes a module's top-level code during import before failing to
  bind a missing name from it - confirming `src.core.main`'s own
  top-level code is what actually runs the tool, not the broken
  `entry_point` function.
  Fixed by invoking `commix.py` (the repo-root script, confirmed to be
  the tool's own real intended entry point - it checks `if __name__ ==
  '__main__'` and does a plain module-level import of `src.core.main`)
  directly via `python3`, bypassing the broken pip console-script
  entirely. This also meant `pip install` wasn't needed at all - no
  `requirements.txt` exists in the repo and `setup.py` declares no
  `install_requires`, confirming it's a genuinely self-contained
  script needing only its own bundled code and the standard library.
  Verified: `cargo build --locked --release --bins` and `cargo test
  --lib` (12/12) clean. Live re-verification pending redeploy.

- Fixed a second real `commix` bug, found immediately after redeploying
  the fix above: the crash was gone (exit code 0), but output still
  stopped right after "Using 'stdin' for parsing targets list." -
  never actually testing the given URL at all. Traced to a real,
  confirmed quirk in commix's own source: it checks whether stdin is a
  TTY, and since this fleet's containers always run with a non-TTY
  stdin, it silently falls into reading targets from stdin instead of
  respecting `-u`, unless `--ignore-stdin` is also passed - regardless
  of whether a URL was already given. Added `--ignore-stdin` to
  `fixed_args`, so `-u` is actually honored every time. Second real bug
  found in this one plugin through live testing alone, neither visible
  from reading `setup.py`/`menu.py` in isolation - both needed an
  actual run to surface.
  Verified: `cargo build --locked --release --bins` and `cargo test
  --lib` (12/12) clean. Live re-verification pending redeploy.
