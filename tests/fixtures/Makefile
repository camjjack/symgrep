# ELF fixtures (build on Linux). The committed binaries let the tests run on
# any host, since goblin parses each format regardless of the host OS.
all: main

libtest.so: libtest.c
	gcc -shared -fPIC -o $@ $<

main: main.c libtest.so
	gcc -o $@ $< -L. -ltest

# Mach-O fixtures (build on macOS).
macho: main_macho

libtest.dylib: libtest.c
	cc -dynamiclib -o $@ $<

main_macho: main.c libtest.dylib
	cc -o $@ $< -L. -ltest
