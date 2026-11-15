# Moon Base 101 — księżycowa ekspedycja

Gra desktopowa w Rust / Bevy 0.13.2: eksploracja izometrycznego Księżyca,
wydobycie, budowanie bazy, przetrwanie i projektowanie misji.

Moon Base 101 starts in English. Choose Polish under **Settings → Language**.
The mobile client remembers its own language setting. Both editions use the
same message catalogs in `locales/en.json` and `locales/pl.json`. Keep catalog keys
and format placeholders stable when editing the translations.

[Szczegółowa dokumentacja gry](DOKUMENTACJA.md) opisuje technologie, mechaniki, sieć i strukturę kodu.

[Wersje Linux X11/Vulkan i Wayland/OpenGL](LINUX.md). Skrypt `sh run.sh --join ADRES_IP:7878` uruchamia grę i rozpoczyna połączenie z hostem Windows.

Pełny ekran przełącza się przyciskiem na górze ustawień lub klawiszem **F11**.
Przytrzymaj **Tab**, aby zobaczyć graczy i ping. **Enter** otwiera wspólny czat.
Przycisk **Rozbiórka** wybiera niszczenie budynków z odzyskiem materiałów na ziemi.
Klient WWW korzysta z tekstur aplikacji, pokazuje pozostałych graczy i udostępnia
wspólny czat. Zamknięcie hosta kończy sesję klientów. Host i klienci desktopowi
powinni korzystać z tej samej aktualizacji.

W buildzie **ORBIT-011** klient WWW korzysta z WebSocket. W ustawieniach gry można
zmienić imię, skróty i limit FPS. Czat zachowuje do 50 wiadomości. Przy otwartym
czacie przewijaj historię rolką albo klawiszami Page Up i Page Down.

- **F4** ukrywa HUD w trakcie gry. Wciśnij ponownie, aby go przywrócić.
- **H** stawia flagę na wolnym polu obok astronauty. Na telefonie wybierz **Postaw flagę** w plecaku.
- Flara po lądowaniu oświetla 5 × 5 pól przez 11 sekund. Kolejną możesz rzucić po 6 sekundach.
- Przytrzymaj LPM na wraku LUR-1 lub wybierz wrak i przytrzymaj **Kop** na telefonie. Naprawa kosztuje 3 szt. krzemu i 7 szt. regolitu.
- Warsztat pozwala ulepszyć plecak z 35 kg do 110 kg. W Sandboxie możesz dodawać przedmioty w plecaku i wyłączyć zadania w kreatorze misji.

## Uruchomienie

Na tym komputerze kliknij dwukrotnie **Uruchom-MoonBase101.cmd** w folderze `E:\giera`.
Skrypt uruchamia gotowy plik MoonBase101.exe. Nie wymaga instalacji Rusta.

Na innym komputerze z Rust i narzędziami linkera, w katalogu `LunaGame`:

```powershell
cargo run --locked
```

## Sterowanie

| Klawisz | Działanie |
|---|---|
| WASD / strzałki | Ruch |
| Spacja / Shift | Skok / plecak odrzutowy w powietrzu |
| Przytrzymanie LPM | Kopanie albo rozbiórka w trybie kilofa |
| 1–0, LPM | Wybór modułu i budowanie |
| B / Q | Zmiana trybu kilof / budowa |
| E | Wejście / wyjście z bazy lub łazika |
| G przy stacji | Wejście do kokpitu lub wyjście po lądowaniu |
| WASD / Spacja / Ctrl w kokpicie | Lot poziomy / wznoszenie / lądowanie |
| N | Dziennik odkrywcy i wybór planety w kokpicie |
| P | Zbadaj pobliski fioletowy punkt badań |
| I / M / J / K | Plecak / mapa / postęp / postać |
| R / T | Zjedz posiłek / napij się wody |
| C / X | Ulepszenie narzędzia / apteczka |
| Z / H | Zmiana członka załogi AI / flaga |
| Rolka, + / − | Przybliżenie / oddalenie |
| O | Radar orbitalny |
| Esc | Zamknięcie panelu, pauza lub wznowienie |

W ustawieniach można włączyć przyciski ekranowe. Przycisk „AKCJA” trzeba
przytrzymać, aby dokończyć kopanie.

Na pierwszą wyprawę dostajesz dwie racje i dwie porcje wody. Dalsze zapasy
są w kapsułach. Wejdź do zasilonej bazy i wybierz „Zrób wodę”, aby zamienić 1 lód
na 2 porcje wody. „Zrób posiłek” zużywa 1 porcję wody i wymaga podłączonej
szklarni. W trybie łatwym pierwsza baza ma już szklarnię. W razie braku prądu
przygotowanie się zatrzyma i wznowi po jego przywróceniu. Panel skafandra pokazuje sześć wskaźników i szacuje
czas pozostały do opróżnienia butli. Kopanie, ruch i nieszczelny skafander
zwiększają zużycie tlenu. W zasilonej bazie z generatorem uzupełnisz butlę.

## Wydobycie i atlas geologiczny

Złoża mają skończony zapas. Płaski kafelek rudy maleje wraz z paskiem kopania,
a wyczerpane złoże znika, pozostawiając grunt. Po cyklu wypadają odłamki.
Podejdź do nich pieszo, aby zebrać surowce i próbki — samo kopanie nie dodaje ich
do plecaka. Odłamki pozostają w świecie podczas bieżącej wyprawy, również po
odejściu z sektora. Nie można zbierać ich z wnętrza bazy ani łazika.

Dodatkowy cel **Atlas geologiczny**: zbierz lód, tytan, hel-3 i rzadkie minerały,
a następnie wejdź do bazy lub lądownika. Jednorazowo otrzymasz raport 120 GB
do transmisji. Postęp 0–4 jest widoczny w górnym panelu misji.

Podejdź do bazy lub łazika i naciśnij **E**. Przy stacji naciśnij **G**, aby wejść
do kokpitu. Przytrzymaj spację, aby wznieść się ponad teren, a potem leć WASD.
Przytrzymaj Ctrl nad wolnym, płaskim miejscem. Po lądowaniu użyj G, aby wysiąść.
W mapie **M** biały kwadrat wskazuje gracza, a turkusowy stację. Kliknięcie odkrytego
pola dodaje znacznik.

## Misje i zapis

„Graj” otwiera odprawę misji „Raport dla Ziemi”. Przygotuj wodę i jedzenie,
uruchom antenę, zbadaj po cztery punkty na Księżycu, Marsie i Wenus,
a następnie wyślij raport z bazy na Księżycu. Złota strzałka wskazuje bieżący cel.
**P** rozpoczyna pomiar przy znaczniku albo obsługuje antenę w bazie.
Do odległych punktów jedź łazikiem i wysiądź przed pomiarem. Tempo zależy
od trasy i postojów. Misja jest zaprojektowana na około 20 minut, bez licznika
odliczającego czas do porażki.
„Zaprojektuj misję” pozwala wybrać trudność, scenariusz, wyposażenie i postać.

W trybie solo (lub jako host) wejdź do kokpitu [G], otwórz dziennik [N] i wybierz Marsa, Wenus
lub powrót na Księżyc. Mars ma burze pyłowe, a Wenus skrajny upał i gęstą atmosferę. Są też
kratery. Każdy świat ma cztery punkty badań [P], również w trybie własnym. Budynki, stan złóż, pojazdy, odłamki
i odkrycia zachowują się podczas podróży w obrębie jednej wyprawy. Lot, pogoda
i zużycie energii są uproszczone na potrzeby gry.

Buduj moduły bokami obok siebie, aby połączyć ich zasilanie. Panel i generator
ładują akumulatory w habitacie oraz magazynie. Generator tlenu działa w sieci
z prądem. Zasilone laboratorium analizuje próbki, a szklarnia przygotowuje
zamówione posiłki. HUD w środku pokazuje prąd, tlen i zapas
baterii.

Materiały do dziennika: [Księżyc](https://science.nasa.gov/moon/facts/),
[Mars](https://science.nasa.gov/mars/facts/), [Wenus](https://science.nasa.gov/venus/venus-facts/),
[grawitacja](https://nssdc.gsfc.nasa.gov/planetary/factsheet/),
[promieniowanie kosmiczne](https://www.nasa.gov/hrp/hazard-space-radiation/).
Zapis / odczyt `mission_design.json` dotyczy **projektu misji**, nie świata ani
postępu rozgrywki. Powrót z ustawień do pauzy zachowuje bieżącą misję.
Menu główne i zmiana trybu kończą bieżącą wyprawę.

## LAN

W menu głównym pierwszego komputera wybierz „Hostuj na PC · LAN i telefon”.
Host tworzy serwer w procesie gry i rozpoczyna misję. Na drugim komputerze
otwórz ustawienia, wpisz IPv4 hosta wyświetlane pod F10 i wybierz „Dołącz”.
Domyślny port to TCP 7878. Dwa okna na jednym komputerze łączą się przez
127.0.0.1:7878. Na telefonie otwórz adres WWW spod F10 w przeglądarce.
Maksymalnie grają cztery osoby łącznie. W Windows wybierz „Zezwól na LAN”
w menu głównym lub pod F10 i potwierdź zgodę administratora. Reguła dotyczy
bieżącego EXE i połączeń TCP 7878 i 7879 z lokalnej podsieci.

Plecak [I] pozwala wyrzucić jedną sztukę lub cały stos na otwartym terenie.
Przedmiot może podnieść inny gracz na tej samej planecie. Otwarty plecak
ukrywa kartę zadań. Poza plecakiem użyj przycisku „Zwiń zadania”.

Host ustala pogodę, zagrożenia, stan terenu, złoża, budowle, odłamki,
magazyn, pomiary i postęp misji. Zatwierdza kopanie, budowanie, crafting,
podnoszenie przedmiotów i wspólne emotki. Ruch astronauty jest lokalny,
a inni gracze oglądają wygładzone aktualizacje; fizjologia skafandra jest
liczona lokalnie na podstawie pogody hosta. Każdy ma własny plecak i parametry
przetrwania. Każdy podróżuje osobno przy stacji [N]; pozostali zostają na swojej planecie. Esc w sieci otwiera menu,
a symulacja świata nadal działa. Wyjście hosta kończy serwer.

F5 — powitanie, F6 — taniec, F7 — radość. Podejdź do gracza i naciśnij F8,
aby zaprosić do piątki, albo F9, aby zaprosić do tańca. Drugi gracz dołącza
tym samym klawiszem w ciągu 8 sekund. Piątka ma wspólną animację i efekt dźwiękowy.

Klient mobilny jest osadzony w grze i dostępny przez przeglądarkę telefonu
w tej samej sieci. Host uruchamia stronę na porcie TCP 7879 podczas sesji.

## Zmiany i sprawdzanie

- Nowe menu ekspedycji, spójna paleta, lżejsza typografia i skalowanie UI.
- Poprawione klikanie HUD, ukrywanie paneli, przyciski zoomu i powrót z ustawień.
- Poprawiony zwrot kosztów budowy, podgląd dostępności i reset nowej misji.
- Ograniczenie liczby odległych obiektów renderowanych bez utraty danych mapy.
- Komunikacja zwrotna LAN, widoczność dołączających graczy i kontrola pakietów.
- Czytelne błędy zapisu / odczytu projektu oraz normalizacja wczytanych danych.

```powershell
cargo fmt --check
cargo test --locked
cargo clippy --locked
```

Oryginał źródeł sprzed poprawek znajduje się w `.backup-original`.

Autorzy gry: _wojtasz_, f57ers, hajaxu, melodyxmemorie, rafi2332, wrfghui.

NASA does not endorse any non-U.S. Government entity and is not responsible for
information contained on non-U.S. Government websites.

## Aktualizacja ekspedycji i TCP — 14 listopada 2026

- Plecak: 35 kg. Nadmiar nagród wypada na ziemię. Przy magazynie można odłożyć
  ładunek i odebrać go przyciskami w bazie. Magazyn jest wspólny w sesji.
- Budynki kosztują około 50% więcej, narzędzia około 20% więcej. Wszystkie
  receptury narzędzi mieszczą się w limicie plecaka.
- Woda wymaga stanowiska przy elektrolizerze; posiłki — szklarni. Śluza nie
  jest stanowiskiem produkcji. Zasilanie i rezerwowanie składników nadal obowiązują.
- Rzadkie złoża tworzą proceduralne skupiska. Losowe struktury i skrzynie
  występują znacznie rzadziej. Planety mają odrębne generatory wysokości,
  układy kraterów, zasoby i pozycje badań. Powroty zachowują ich stan.
- Po zagrożeniu jest 90–150 sekund spokoju (120–180 na łatwym poziomie).
  Burze słoneczne uszkadzają odsłonięte skafandry na wszystkich trzech światach.
- Flara ma 6 sekund odnowienia. Host egzekwuje ten sam limit.
- Zmiana astronauty Z działa na ziemi poza kabiną; zachowuje ekwipunek i
  odrzuca niebezpieczne pozycje. W sieci każdy steruje własną postacią.
- Opisy obiektów pod kursorem, znikające nazwy zebranych przedmiotów i
  schemat ośmiu planet Układu Słonecznego z jasnym Słońcem.

Testy: cargo test --locked. Budowa: cargo build --locked.

## Równoległe światy i pasażerowie

Serwer utrzymuje odrębne aktywne światy Księżyca, Marsa i Wenus. Przy stacji
otwórz N i wybierz własny cel podróży. Inni gracze pozostają tam, gdzie byli.
Teren, pojazdy, produkcja, magazyn i pogoda należą do konkretnej planety;
misja i odkrycia są wspólne. Zmiana planety przez hosta zachowuje świat,
w którym nadal grają pozostali. Kierunkowskazy pokazują załogę na Twojej planecie.

E przy łaziku zajmuje wolne miejsce: pierwsza osoba prowadzi, następne jadą
jako pasażerowie. Lekki łazik mieści 2 osoby, ciężki 4. Tylko kierowca steruje
WASD; E pozwala wysiąść w bezpiecznym miejscu. Po wyjściu kierowcy pierwszy
pasażer przejmuje prowadzenie. Pod kursorem widać liczbę zajętych miejsc.

## Zdjęcie dnia NASA w menu

Menu pobiera APOD w osobnym wątku: tytuł, datę, typ, opis, podgląd oraz odnośnik
HD, jeśli NASA go udostępnia. Pełny opis jest przewijany rolką; źródłowy tekst
NASA pozostaje po angielsku. Filmy otwiera się na stronie NASA. Brak Internetu,
limit API lub błąd NASA nie blokuje rozpoczęcia gry. Odświeżenie ma 30 sekund
przerwy, żeby uniknąć wielokrotnych zapytań. Panel zachowuje dane w pamięci sesji.

Klucz jest odczytywany ze zmiennej NASA_API_KEY albo pliku nasa_api_key.txt obok
pliku EXE; domyślnie działa DEMO_KEY. Prywatny plik klucza nie trafia do archiwum
źródeł. Windows używa systemowego HTTPS curl.exe i dekodera obrazów, bez instalacji
kolejnych bibliotek. API może czasowo zwracać HTTP 500; pokazujemy wtedy komunikat.

W ostatniej kompilacji Windows zaliczono 43 testy. Test z aktualnymi danymi NASA
APOD jest domyślnie pomijany i wymaga połączenia z NASA.

Gdy APOD API zwróci ogólne logo NASA lub błąd, gra odczytuje aktualny artykuł
z publicznej strony science.nasa.gov/apod/, wybierając obraz wewnątrz artykułu.
Tytuł, data, opis i autor obrazu pozostają powiązane z tym samym odkryciem.
