rem Set the paths appropriately
PATH C:\msys64\%MSYSTEM%\bin;C:\msys64\usr\bin;%PATH%

rem Upgrade the MSYS2 platform
bash -lc "pacman --noconfirm --sync --refresh pacman"
bash -lc "pacman --noconfirm --sync --refresh --sysupgrade"

rem Install required tools
rem bash -xlc "pacman --noconfirm -S --needed base-devel"

rem Install the relevant native dependencies
bash -xlc "pacman --noconfirm -S --needed pacman-mirrors"
bash -xlc "pacman --noconfirm -S --needed diffutils make mingw-w64-%MSYS2_ARCH%-gcc"

rem Build Whitenoise
bash -xlc "cd runtime-rust/; cargo build"