# Iris GDTF Share

HTTP service for sharing GDTF fixture profiles. Community uploads are open to everyone;
the **official Iris database** is managed with an administrator token.

```sh
GDTF_SHARE_ADMIN_TOKEN='a-long-random-secret' GDTF_SHARE_ADDR=0.0.0.0:8787 cargo run --release
```

| Variable | Default | Purpose |
|---|---|---|
| `GDTF_SHARE_DATA` | `data` | Storage directory (`index.json` + `files/`) |
| `GDTF_SHARE_ADDR` | `127.0.0.1:8787` | Listen address |
| `GDTF_SHARE_ADMIN_TOKEN` | unset | ≥12 chars; admin endpoints are disabled without it |

Open `/` for a web page to browse, upload, and (with the token) add official profiles,
promote/demote community files, or delete anything.

## API (`/api/v1`)

- `GET /profiles?q=&source=official|community|all`, `GET /profiles/{id}`, `GET /profiles/{id}/download`
- `POST /profiles` — raw `.gdtf` body, optional `X-Uploader`; returns a one-time `delete_token`
- `DELETE /profiles/{id}` with `X-Delete-Token`
- Admin (`Authorization: Bearer <token>`): `POST /admin/profiles` (official upload),
  `POST /admin/profiles/{id}/promote|demote`, `DELETE /admin/profiles/{id}`

Uploads are validated (zip limits, `description.xml`, single fixture type) and de-duplicated by SHA-256.
Put the server behind a TLS reverse proxy with rate limiting for public use. In Iris, open
**Patch → GDTF Share** and set the server address.
