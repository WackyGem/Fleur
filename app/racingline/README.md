# Racingline

Racingline is the Rearview frontend workbench for strategy research, strategy backtesting, and strategy portfolio monitoring.

## Local Development

Use the repository Makefile from the repo root:

```bash
make racingline-dev
```

This starts Rearview, the portfolio worker, and the Vite dev server at `http://127.0.0.1:5173/`.

To run only the frontend after Rearview is already available:

```bash
make racingline-frontend-dev
```

## Quality Gates

```bash
npm run lint
npm run typecheck
npm test
npm run build
```

The app reads `VITE_REARVIEW_API_BASE_URL` from the repository root `.env` / `.env.example` through Vite `envDir`.
