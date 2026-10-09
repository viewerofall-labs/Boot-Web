#!/usr/bin/env bash
# Builds esp/initramfs.gz: a tiny busybox shell.
set -euo pipefail
cd "$(dirname "$0")"

BUSYBOX="${BUSYBOX:-$(command -v busybox)}"
rm -rf build/initramfs
mkdir -p build/initramfs/{bin,sbin,usr/bin,usr/sbin,dev,proc,sys,root,tmp,etc}

cp "$BUSYBOX" build/initramfs/bin/busybox
chmod +x build/initramfs/bin/busybox

# busybox must be statically linked, or nothing in the initramfs can run it.
ldd_out="$(ldd "$BUSYBOX" 2>&1 || true)"
case "$ldd_out" in
  *"not a dynamic"*|*"statically linked"*) ;;
  *) echo "warning: $BUSYBOX looks dynamically linked; it will not run in the initramfs" >&2 ;;
esac

# Do NOT use `busybox --install -s`: it points every symlink at the absolute
# *host* path of the binary (e.g. /home/you/boot/build/initramfs/bin/busybox),
# which does not exist inside the initramfs, so /bin/sh becomes "not found".
# Make the links ourselves, pointing at /bin/busybox as seen from inside.
(
  cd build/initramfs
  applets="$(./bin/busybox --list-full 2>/dev/null || true)"
  if [ -z "$applets" ]; then
    applets="$(./bin/busybox --list | sed 's|^|bin/|')"
  fi
  printf '%s\n' "$applets" | while read -r applet_path; do
    [ "$applet_path" = "bin/busybox" ] && continue
    mkdir -p "$(dirname "$applet_path")"
    ln -sf /bin/busybox "$applet_path"
  done
)

# Sanity check: fail loudly here instead of at boot.
[ "$(readlink build/initramfs/bin/sh)" = "/bin/busybox" ] || {
  echo "error: bin/sh is not a link to /bin/busybox" >&2
  exit 1
}

cat > build/initramfs/init <<'INIT'
#!/bin/busybox sh
export PATH=/bin:/sbin:/usr/bin:/usr/sbin
export HOME=/root
export USER=root
export PS1='htmlboot:\w# '

mkdir -p /proc /sys /dev /tmp /run /root /etc
mount -t proc proc /proc 2>/dev/null || true
mount -t sysfs sysfs /sys 2>/dev/null || true
mount -t devtmpfs devtmpfs /dev 2>/dev/null || true

# Fallback device nodes if devtmpfs is unavailable
[ -e /dev/console ] || mknod -m 600 /dev/console c 5 1
[ -e /dev/null ] || mknod -m 666 /dev/null c 1 3
[ -e /dev/zero ] || mknod -m 666 /dev/zero c 1 5
[ -e /dev/tty ] || mknod -m 666 /dev/tty c 5 0
[ -e /dev/tty1 ] || mknod -m 620 /dev/tty1 c 4 1
[ -e /dev/ttyS0 ] || mknod -m 660 /dev/ttyS0 c 4 64

clear 2>/dev/null || true
echo "========================================="
echo "  Welcome to HTMLBoot Linux (initramfs)  "
echo "========================================="
echo "Interactive root shell ready. Type commands or exit to restart."
echo

# Start interactive shell and loop to prevent kernel panic on exit
while true; do
    setsid cttyhack /bin/sh 2>/dev/null || /bin/sh
    echo "Shell exited. Restarting in 1s..."
    sleep 1
done
INIT
chmod +x build/initramfs/init

mkdir -p esp
(cd build/initramfs && find . | cpio -o -H newc | gzip -9) > esp/initramfs.gz
echo "Wrote esp/initramfs.gz successfully."
