# Examples

Small Programs that build with the `draconic` CLI.

| Example | Path | Target | What it shows |
|---------|------|--------|----------------|
| FizzBuzz | [`fizzbuzz/`](./fizzbuzz/) | JS (+ Node) | Control flow + strings; clone → build → run |
| HTTP echo | [`http-echo/`](./http-echo/) | native | Pure Draconic HTTP/1.1 listen/accept (no C host) |
| Todo | [`todo/`](./todo/) | JS (browser) + C static host | DOM / `localStorage` via `globalThis` |
| pkg-lib | [`pkg-lib/`](./pkg-lib/) | package | Minimal exportable git module (`draconic.toml` + `index.drac`) |
| pkg-consumer | [`pkg-consumer/`](./pkg-consumer/) | package | Depends on pkg-lib via module path; documented get/tidy + build |
