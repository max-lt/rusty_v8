import platform
import os
import tempfile
import tarfile
import sys
import subprocess
import shutil

DIR = 'third_party/rust-toolchain'
SENTINEL = f'{DIR}/.rusty_v8_version'

host_os = platform.system().lower()
if host_os == "darwin":
    host_os = "mac"
elif host_os == "windows":
    host_os = "win"

host_cpu = platform.machine().lower()
if host_cpu == "x86_64":
    host_cpu = "x64"
elif host_cpu == "aarch64":
    host_cpu = "arm64"


def setup_system_rust_toolchain():
    """On ARM64 Linux, create a toolchain dir with symlinks to system Rust."""
    print(f'{DIR}: ARM64 Linux detected, using system Rust toolchain')

    # Get system Rust sysroot
    try:
        sysroot = subprocess.check_output(
            ['rustc', '--print', 'sysroot'],
            text=True
        ).strip()
    except Exception as e:
        print(f'Error: Could not find system rustc: {e}')
        sys.exit(1)

    print(f'{DIR}: System Rust sysroot: {sysroot}')

    # Remove existing directory/symlink
    if os.path.islink(DIR):
        os.unlink(DIR)
    elif os.path.exists(DIR):
        shutil.rmtree(DIR)

    # Create the directory structure
    os.makedirs(DIR, exist_ok=True)
    bin_dir = os.path.join(DIR, 'bin')
    os.makedirs(bin_dir, exist_ok=True)

    # Symlink lib directory from system sysroot
    sysroot_lib = os.path.join(sysroot, 'lib')
    if os.path.exists(sysroot_lib):
        os.symlink(sysroot_lib, os.path.join(DIR, 'lib'))
        print(f'{DIR}: Symlinked lib -> {sysroot_lib}')

    # Symlink all binaries from system sysroot bin
    sysroot_bin = os.path.join(sysroot, 'bin')
    if os.path.isdir(sysroot_bin):
        for item in os.listdir(sysroot_bin):
            src = os.path.join(sysroot_bin, item)
            dst = os.path.join(bin_dir, item)
            os.symlink(src, dst)
        print(f'{DIR}: Symlinked binaries from {sysroot_bin}')

    # Ensure bindgen is available
    bindgen_dst = os.path.join(bin_dir, 'bindgen')
    if os.path.exists(bindgen_dst):
        print(f'{DIR}: bindgen already linked')
        return

    # Check if bindgen is installed via cargo
    bindgen_path = shutil.which('bindgen')
    if not bindgen_path:
        print(f'{DIR}: bindgen not found, installing via cargo...')
        subprocess.run(['cargo', 'install', 'bindgen-cli'], check=True)
        bindgen_path = shutil.which('bindgen')

    if bindgen_path:
        print(f'{DIR}: Found bindgen at {bindgen_path}')
        os.symlink(bindgen_path, bindgen_dst)
        print(f'{DIR}: Symlinked bindgen')
    else:
        print(f'{DIR}: ERROR: Could not find or install bindgen')
        sys.exit(1)


# ARM64 Linux: use system toolchain
if host_os == "linux" and host_cpu == "arm64":
    setup_system_rust_toolchain()
    sys.exit(0)

# Other platforms: download Chromium's prebuilt toolchain
from v8_deps import deps
from download_file import DownloadUrl

eval_globals = {
    'host_os': host_os,
    'host_cpu': host_cpu,
}

dep = deps[DIR]
obj = next(obj for obj in dep['objects'] if eval(obj['condition'], eval_globals))
bucket = dep['bucket']
name = obj['object_name']
url = f'https://storage.googleapis.com/{bucket}/{name}'


def EnsureDirExists(path):
    if not os.path.exists(path):
        os.makedirs(path)


def DownloadAndUnpack(url, output_dir):
    """Download an archive from url and extract into output_dir."""
    with tempfile.TemporaryFile() as f:
        DownloadUrl(url, f)
        f.seek(0)
        EnsureDirExists(output_dir)
        with tarfile.open(mode='r:xz', fileobj=f) as z:
            z.extractall(path=output_dir)


try:
    with open(SENTINEL, 'r') as f:
        if f.read() == url:
            print(f'{DIR}: already downloaded')
            sys.exit()
except FileNotFoundError:
    pass

DownloadAndUnpack(url, DIR)

with open(SENTINEL, 'w') as f:
    f.write(url)
