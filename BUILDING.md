# Building instructions #

Currently, only Linux is supported as a development platform. That doesn't mean
Nucloid can't be built on other operating systems, just that is hasn't been
tested, and some adjustments are almost certainly required. On Windows, it may
be a good idea to go with WSL.

## rustup ##

You need rustup to be installed. If it isn't, run:

```sh
$ curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Or checkout your distribution for a package: `rustup` on Arch Linux; Debian does
not package rustup.

You then need to install the `nightly-x86_64-unknown-linux-gnu` toolchain:

```sh
$ rustup toolchain install nightly-x86_64-unknown-linux-gnu
```

As well as the standard library's sources:

```sh
$ rustup component add rust-src --toolchain nightly-x86_64-unknown-linux-gnu
```

## GCC x86-64 ELF cross-compiler ##

You need a GCC cross-compiler `x86_64-elf-gcc`.

On Arch Linux, just install the AUR package `x86_64-elf-gcc`.

### Debian, and other distributions ###

Debian does not ship a package for the `x86_64-elf-gcc` toolchain, unlike Arch
Linux. You will have to build it yourself from the sources. Note that this
method is not strictly limited to Debian and can work for any distribution.

You first need to install dependendies needed to build GCC and the binutils;
for Debian, these are the packages to install:

```sh
# apt install libgmp3-dev libmpc-dev texinfo
```

You can download the sources of binutils and GCC from GNU's website. For this
tutorial, I will take binutils v2.40 and GCC v12.2; feel free to use any later
version.

Note: you must build the binutils first.

```sh
wget https://ftp.gnu.org/gnu/binutils/binutils-2.40.tar.xz
tar xfJ binutils-2.40.tar.xz
mkdir build_binutils && cd build_binutils
../binutils-2.40/configure --target=x86_64-elf --prefix=/usr/local --with-sysroot --disable-werror
make -j16
sudo make install
```

Do not run `make -j`: this does not limit the number of concurrent compilation
processes and will consume an enormous amount of RAM with no performance
benefit (on my setup, it immediately filled up the 32 GiB of RAM and the 8 GiB
of swap, triggering the OOM killer).

Then for GCC:

```sh
wget https://ftp.gnu.org/gnu/gcc/gcc-12.2.0/gcc-12.2.0.tar.xz
tar xfJ gcc-12.2.0.tar.xz
mkdir build_gcc && cd build_gcc
../gcc-12.2.0/configure --target=x86_64-elf --prefix=/usr/local --without-headers
make -j16 all-gcc
make -j16 all-target-libgcc
sudo make install-gcc
sudo make install-target-libgcc
```

You can then delete the two build directories.

## Rust build ##

Once rustup and your cross-compiler toolchain are installed, you can trigger the
Rust compilation via make:

```sh
make x86_64-debug
```

For the debug version, or use the `x86_64-release` target for the release build.

This will generate the output kernel ELF in the `target` directory:
`target/x86_64-nucloid/debug/nucloid` for the debug build, and
`target/x86_64-nucloid/release/nucloid` for the release.
