from setuptools import setup
import os

# turn on backtraces in rust (for build.rs)
os.environ['RUST_BACKTRACE'] = 'full'  # '1'
os.environ['RUSTFLAGS'] = ""

# set the environment variable to increase compiler optimization
release = os.environ.get("WHITENOISE_RELEASE") == "True"

rust_build_path = os.path.join('target', 'release' if release else 'debug')

rust_build_cmd = 'cargo build'
if release:
    rust_build_cmd += ' --release'

bash_build_cmd = ['bash', '-c', rust_build_cmd]


def build_native(spec):
    build_validator = spec.add_external_build(
        cmd=bash_build_cmd,
        path=os.path.join('..', 'validator-rust')
    )

    spec.add_cffi_module(
        module_path='whitenoise._native_validator',
        dylib=lambda: build_validator.find_dylib('whitenoise_validator', in_path=rust_build_path),
        header_filename=lambda: build_validator.find_header('api.h', in_path='.'),
        rtld_flags=['NOW', 'NODELETE']
    )

    build_runtime = spec.add_external_build(
        cmd=bash_build_cmd,
        path=os.path.join('..', 'runtime-rust')
    )

    spec.add_cffi_module(
        module_path='whitenoise._native_runtime',
        dylib=lambda: build_runtime.find_dylib('whitenoise_runtime', in_path=rust_build_path),
        header_filename=lambda: build_runtime.find_header('api.h', in_path='.'),
        rtld_flags=['NOW', 'NODELETE']
    )


def build_python(spec):
    spec.add_external_build(
        cmd=['bash', '-c', 'python3 code_generation.py'],
        path="."
    )

setup(
    package_dir={"whitenoise": "whitenoise"},
    package_data={"whitenoise": ["variant_message_map.json"]},
    milksnake_tasks=[
        build_native,
        build_python
    ]
)

