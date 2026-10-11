# cronwhen

Explains a cron expression in plain English and lists its next run
times, entirely offline. The alternative today is pasting a crontab
line into crontab.guru — a website — every time you need to sanity-check
a schedule before deploying it. This is the same check as a local
binary, scriptable and CI-friendly.

## Installation

```sh
cargo install --path .
```

## Usage

```bash
cronwhen "*/15 * * * *"
cronwhen "30 9 * * *" -n 10        # show 10 upcoming runs instead of the default 5
cronwhen "@weekly"                  # @yearly/@monthly/@weekly/@daily/@hourly all work
```

```
$ cronwhen "*/15 * * * *"
every 15 minutes

next 5 run(s):
  2026-08-24 20:30:00 Mon
  2026-08-24 20:45:00 Mon
  2026-08-24 21:00:00 Mon
  2026-08-24 21:15:00 Mon
  2026-08-24 21:30:00 Mon
```

Standard 5-field cron syntax: `minute hour day-of-month month
day-of-week`. Supports `*`, ranges (`1-5`), steps (`*/15`, `1-20/2`),
comma lists (`MON,WED,FRI`), and month/weekday names. Implements cron's
well-known but frequently-misunderstood rule that when **both**
day-of-month and day-of-week are restricted, a day matches if it
satisfies *either* one, not both (e.g. `0 0 13 * FRI` means "the 13th
of the month, or any Friday" — not "Friday the 13th only").

## Status: built and verified

- **20 unit tests** across field parsing (wildcards, steps, ranges,
  comma lists, name resolution, the `0`/`7`-both-mean-Sunday quirk),
  next-run scheduling (minute rollover, day rollover, the day-of-month/
  day-of-week OR rule), and the English describer (every-N-minutes,
  daily-at-a-fixed-time, specific weekdays, specific day-of-month, and
  the plain-field fallback for shapes that don't get a hand-written
  sentence).
- **Live-run against real-world schedules** and manually cross-checked
  against known cron semantics: `*/5 * * * *` correctly lands on
  `:20/:25/:30`-style boundaries from an arbitrary start time; `30 9 * *
  *` correctly rolls to the next calendar day when today's 9:30 has
  already passed; `0 0 13 * FRI` was run 5 runs deep and produced both
  a bare Friday (Sept 4) and a bare 13th-that-isn't-a-Friday (Sept 13,
  a Sunday) — proof the OR rule is real, not just AND with extra steps;
  `@weekly` correctly expanded to `0 0 * * 0` and landed on consecutive
  Sundays. A malformed expression (`nonsense`) fails with a clear error
  and a non-zero exit code, not a panic.

**Deliberate scope limits, stated plainly**: no timezone support (runs
entirely in local time via `chrono::Local`, matching how system cron
itself behaves without an explicit `CRON_TZ`); no 6-field
seconds-precision cron (Quartz-style); the plain-English describer
covers the common shapes and falls back to a structured per-field
readout for anything unusual rather than guessing at a sentence that
might be wrong — see `src/describe.rs` for exactly which shapes get
which. The next-run search has a 5-year window; a schedule whose next
occurrence falls further out than that (a `Feb 29`-only schedule run
right after a leap year, for instance) reports "not found" rather than
hanging.
