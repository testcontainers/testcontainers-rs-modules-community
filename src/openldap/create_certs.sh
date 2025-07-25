#!/bin/bash

## Helper Script for creating certificates for the LDAP server tls tests

# Set variables
ROOT_SUBJECT="/C=AT/O=Test/OU=Testcontainer/CN=rootCA"
SERVER_SUBJECT="/C=AT/O=Test/OU=Testcontainer/CN=ldap.example.org"
DAYS_UNTIL_EXPIRE=$((365 * 20))

# Create directory for certificates
mkdir -p certs
rm -fr certs/*
cd certs

# Create a config file for the server certificate
cat >server.cnf <<EOF
[req]
distinguished_name = req_distinguished_name
x509_extensions = v3_req
prompt = no

[req_distinguished_name]
C = AT
O = Test
OU = Testcontainer
CN = ldap.example.org

[v3_req]
keyUsage = critical, digitalSignature, keyEncipherment
extendedKeyUsage = serverAuth
subjectAltName = @alt_names

[alt_names]
DNS.1 = localhost
DNS.2 = openldap
EOF

# Create a self-signed root certificate
openssl req -x509 -nodes -days $DAYS_UNTIL_EXPIRE -newkey rsa:2048 -keyout rootCA.key -out rootCA.crt -subj "$ROOT_SUBJECT"

# Create a key for the server certificate
openssl genrsa -out ldap.example.org.key 2048

# Generate Server CSR using the config file
openssl req -new -key ldap.example.org.key -out ldap.example.org.csr -subj "$SERVER_SUBJECT"

# Sign the server certificate with the Root CA
openssl x509 -req -in ldap.example.org.csr -CA rootCA.crt -CAkey rootCA.key -CAcreateserial -out ldap.example.org.crt -days $DAYS_UNTIL_EXPIRE -extfile server.cnf -extensions v3_req

# Clean up
rm -r *.csr *.cnf *.srl rootCA.key

cd -
