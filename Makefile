default: build

build:
	cargo build --release

install:
	mkdir -p ${DESTDIR}/usr/bin
	mkdir -p ${DESTDIR}/usr/lib/systemd/system
	cp target/release/genv ${DESTDIR}/usr/bin/genv
	cp target/release/genv-server ${DESTDIR}/usr/bin/genv-server
	cp genv-server.service ${DESTDIR}/usr/lib/systemd/system/

uninstall:
	rm ${DESTDIR}/usr/bin/genv
	rm ${DESTDIR}/usr/bin/genv-server
	rm ${DESTDIR}/usr/lib/systemd/system/genv-server.service

clean:
	rm -r target
