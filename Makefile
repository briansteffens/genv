default: build

build:
	cargo build --release

install:
	mkdir -p ${DESTDIR}/usr/bin
	cp target/release/genv ${DESTDIR}/usr/bin/genv
	cp target/release/genv-server ${DESTDIR}/usr/bin/genv-server

uninstall:
	rm ${DESTDIR}/usr/bin/genv
	rm ${DESTDIR}/usr/bin/genv-server

clean:
	rm -r target
