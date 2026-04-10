# Frontend Experiment

`mesher/frontend-exp` is a product-owned Next.js service surface for the operator app.

## Local development

```bash
npm ci
npm run dev
```

Default local URL: `http://127.0.0.1:3000`

## Production container path

This app ships a production multi-stage Docker build in `mesher/frontend-exp/Dockerfile`.

Build image:

```bash
docker build -t hyperpush-frontend-exp:latest mesher/frontend-exp
```

Run container:

```bash
docker run --rm -p 3000:3000 \
	-e PORT=3000 \
	-e HOSTNAME=0.0.0.0 \
	--name hyperpush-frontend-exp \
	hyperpush-frontend-exp:latest
```

## Startup contract

Container runtime contract:

- Entrypoint: `node server.js`
- Bind host: `HOSTNAME` (default `0.0.0.0`)
- Listen port: `PORT` (default `3000`)
- Runtime mode: `NODE_ENV=production`

This contract is explicit in the Dockerfile and does not rely on local `next dev` assumptions.

## Generic VM deployment model

The service can run on a generic Linux VM with Docker or a compatible container runtime.

Example systemd unit command shape:

```bash
docker run --rm --pull=always --name hyperpush-frontend-exp \
	-p 3000:3000 \
	-e PORT=3000 \
	-e HOSTNAME=0.0.0.0 \
	hyperpush-frontend-exp:latest
```

## Health verification

After startup, verify service health from the VM:

```bash
curl -fsS http://127.0.0.1:3000/ > /dev/null
```

Exit status `0` confirms the service is serving HTTP traffic.
