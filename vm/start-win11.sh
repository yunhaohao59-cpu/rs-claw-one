#!/bin/bash
set -e

VM_DIR="$(cd "$(dirname "$0")" && pwd)"
ISO="$VM_DIR/Win11_24H2.iso"
DISK="$VM_DIR/win11.qcow2"
OVMF_CODE="/usr/share/OVMF/OVMF_CODE_4M.fd"
OVMF_VARS="$VM_DIR/OVMF_VARS_4M.fd"
TPM_DIR="$VM_DIR/tpm-state"

# ── helpers ──
msg() { echo "  >>> $*"; }

# ── check ISO ──
if [ ! -f "$ISO" ]; then
    echo "❌  Win11 ISO not found: $ISO"
    echo ""
    echo "  Download from Microsoft (choose 'x64 ISO'):"
    echo "  https://www.microsoft.com/software-download/windows11"
    echo ""
    echo "  Place the file as: $ISO"
    exit 1
fi

# ── create disk (thin-provisioned, 60 GB) ──
if [ ! -f "$DISK" ]; then
    msg "Creating 60 GB virtual disk (thin-provisioned)..."
    qemu-img create -f qcow2 "$DISK" 60G
fi

# ── OVMF vars (per-VM copy) ──
if [ ! -f "$OVMF_VARS" ]; then
    msg "Preparing UEFI vars..."
    cp /usr/share/OVMF/OVMF_VARS_4M.fd "$OVMF_VARS"
fi

# ── TPM state ──
if [ ! -d "$TPM_DIR" ]; then
    msg "Initialising TPM emulator..."
    mkdir -p "$TPM_DIR"
    swtpm_setup --tpmstate "$TPM_DIR" --tpm2 --create-ek-cert --create-platform-cert --overwrite
fi

# ── kill old processes ──
pkill -f "swtpm socket.*rs-claw-win11" 2>/dev/null || true

# ── start TPM ──
msg "Starting TPM emulator..."
swtpm socket --tpmstate dir="$TPM_DIR" \
    --ctrl type=unixio,path="$TPM_DIR/swtpm-sock" \
    --tpm2 \
    --pid file="$TPM_DIR/swtpm.pid" \
    --flags not-need-init &
sleep 1

# ── start VM ──
msg "Starting Windows 11 VM..."
echo ""

qemu-system-x86_64 \
    -name "rs-claw-win11" \
    -machine type=q35,accel=kvm \
    -cpu host,kvm=off \
    -smp 4,sockets=1,cores=2,threads=2 \
    -m 8192 \
    \
    -drive if=pflash,format=raw,readonly=on,file="$OVMF_CODE" \
    -drive if=pflash,format=raw,file="$OVMF_VARS" \
    \
    -drive file="$DISK",if=none,id=drive0,format=qcow2 \
    -device ide-hd,drive=drive0,bus=ide.0,bootindex=1 \
    \
    -drive file="$ISO",if=none,id=cdrom,media=cdrom \
    -device ide-cd,bus=ide.0,drive=cdrom,bootindex=2 \
    \
    -tpmdev backend=emulator,id=tpm0,chardev=chrtpm \
    -chardev socket,id=chrtpm,path="$TPM_DIR/swtpm-sock" \
    -device tpm-tis,tpmdev=tpm0 \
    \
    -netdev user,id=net0 \
    -device virtio-net-pci,netdev=net0 \
    \
    -device VGA,vgamem_mb=128 \
    -display gtk \
    -device virtio-tablet-pci \
    \
    -usb -device usb-tablet

# ── cleanup ──
msg "VM stopped. Cleaning up TPM..."
pkill -f "swtpm socket.*rs-claw-win11" 2>/dev/null || true
echo "  Done."
