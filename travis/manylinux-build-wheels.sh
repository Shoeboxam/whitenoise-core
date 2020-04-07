#!/bin/bash

# run from within the manylinux docker containers

# exit immediately upon failure, print commands while running
set -e -x

python3 -m pip install -r /io/bindings-python/requirements.txt
python3 /io/bindings-python/setup.py -d wheelhouse/
