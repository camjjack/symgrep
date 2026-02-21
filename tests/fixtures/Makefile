all: main

libtest.so: libtest.c
	gcc -shared -fPIC -o $@ $<

main: main.c libtest.so
	gcc -o $@ $< -L. -ltest
