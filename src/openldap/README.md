# OpenLDAP Testcontainer

This modules provides the `bitnami/openldap` image as a testcontainer.


## Testing

For testing tls connections, you need to provide a certificate and key. All needed tls artifacts are generated by the `generate-certs.sh` script.

The used openssl version for the latests created certificate is:

```bash
> openssl version
OpenSSL 3.0.13 30 Jan 2024 (Library: OpenSSL 3.0.13 30 Jan 2024)
```

Please update the version here in the README if you generate new certificates for the tests.