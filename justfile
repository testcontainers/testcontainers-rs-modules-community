

test:
  cargo hack test --each-feature --exclude-all-features

install-dev:
  pre-commit install --install-hooks
