# nginx TLS certificates

Place the certificate chain and private key here only if you terminate TLS directly in nginx:

- `fullchain.pem` — certificate plus intermediate chain
- `privkey.pem` — private key

They are mounted read-only at `/etc/nginx/certs` and referenced by the commented TLS server block in `../conf.d/pumapanel.conf`. Files in this directory are git-ignored.
