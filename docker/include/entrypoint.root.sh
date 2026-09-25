#!/usr/bin/bash
runscript() {
	bash $1
	cd /
}
cd /

su -c '/include/entrypoint.rootless.sh' mainuser

source /include/compile-rustctl.sh
