# Moon Base 101 na Linuksie

## Wybierz paczkę

| Paczka | Sesja | Grafika |
| --- | --- | --- |
| `MoonBase101-Linux-X11-Vulkan.tar.gz` | X11 | Vulkan |
| `MoonBase101-Linux-Wayland-OpenGL.tar.gz` | Wayland, w tym Hyprland | OpenGL przez EGL |

Obie paczki są dla 64-bitowego Linuksa x86-64. Potrzebujesz glibc 2.31 lub nowszej i biblioteki ALSA `libasound.so.2`. Wersja X11 wymaga bibliotek X11 oraz działającego sterownika Vulkan. Wersja Wayland wymaga `libwayland-client.so.0`, `libwayland-egl.so.1`, `libxkbcommon.so.0` oraz sterownika z EGL i obsługą OpenGL. Aplikacja Wayland używa identyfikatora `moon-base-101`.

Rozpakuj wybraną paczkę i uruchom w katalogu gry:

```sh
sh run.sh
```

## Połączenie z Windowsem

1. Na Windowsie kliknij **Zezwól na LAN**, a potem **Hostuj na PC · LAN i telefon**.
2. Naciśnij **F10** i odczytaj adres dla gracza na PC. Oba komputery podłącz do tej samej sieci lokalnej.
3. Na Linuksie wpisz adres hosta w ustawieniach gry lub użyj go w terminalu:

```sh
sh run.sh --join 192.168.1.10:7878
```

Wszystkie trzy wydania korzystają z tego samego protokołu TCP na porcie 7878. Telefon nadal może dołączyć przez przeglądarkę pod adresem z F10 na porcie 7879.

## Kompilacja ze źródeł

Rozpakuj `MoonBase101-Zrodla.zip`. Potrzebujesz aktualnego stabilnego Rusta, Cargo, linkera C, `pkg-config` i nagłówków ALSA (`libasound2-dev` na Debianie, `alsa-lib` na Archu). Dla Waylanda biblioteki klienta są ładowane podczas uruchamiania.

```sh
sh run.sh --x11
sh run.sh --wayland
```

Odpowiednie opcje Cargo to `--no-default-features --features linux-x11` oraz `--no-default-features --features linux-wayland`. Każda z nich buduje obsługę jednego systemu okien. Po kompilacji skrypt uruchamia grę i przekazuje argumenty takie jak `--join`.
