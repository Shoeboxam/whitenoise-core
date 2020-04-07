#!/bin/bash

# run from within the manylinux docker containers

# exit immediately upon failure, print commands while running
set -e -x

cd /io/bindings-python/
# python3 -m pip install -r /io/bindings-python/requirements.txt
# python3 /io/bindings-python/setup.py -d wheelhouse/
# Compile wheels
for PYBIN in /opt/python/*/bin; do
    "${PYBIN}/pip" install -r requirements.txt
    "${PYBIN}/python" setup.py bdist_wheel -d wheelhouse/
done
