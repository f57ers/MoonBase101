# Moon Base 101 — księżycowa ekspedycja
## Sadly the whole game is vibe coded but there are plans to actually learn and make something similar without the use of clankers
the game is using primitive shapes for in game objects stuff same with audio it does not exist anywhere its all just noises generated with code i did my best to try and understand the Rust code running all of this but i am a mere beginner

# ENGLISH BELOW

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

# English version here

# Moon Base 101 — Lunar Expedition
## Unfortunately, the entire game is written in Vibe, but there are plans to actually learn and create something similar without using Clankers.
The game uses primitive shapes for in-game objects; the same goes for the audio—it doesn’t exist anywhere; it’s all just noises generated by code. I did my best to try and understand the Rust code running all of this, but I’m just a beginner.

A desktop game in Rust / Bevy 0.13.2: explore an isometric Moon,
mine, build a base, survive, and design missions.

Moon Base 101 starts in English. Choose Polish under **Settings → Language**.
The mobile client remembers its own language setting. Both editions use the
same message catalogs in `locales/en.json` and `locales/pl.json`. Keep catalog keys
and format placeholders consistent when editing the translations.

[Detailed game documentation](DOCUMENTATION.md) describes technologies, mechanics, networking, and code structure.

[Linux X11/Vulkan and Wayland/OpenGL versions](LINUX.md). The script `sh run.sh --join IP_ADDRESS:7878` launches the game and establishes a connection to the Windows host.

Full-screen mode is toggled using the button at the top of the settings menu or the **F11** key.
Hold down **Tab** to view players and ping. **Enter** opens the global chat.
The **Demolish** button selects the option to destroy buildings and recover materials on the ground.
The web client uses the application’s textures, displays other players, and provides
a general chat. Closing the host ends the clients’ sessions. The host and desktop clients
should be running the same update.

In build **ORBIT-011**, the web client uses WebSocket. In the game settings, you can
change your name, hotkeys, and the FPS limit. The chat retains up to 50 messages. With the
chat open, scroll through the history using the scroll wheel or the Page Up and Page Down keys.

- **F4** hides the HUD during gameplay. Press it again to restore it.
- **H** places a flag on an empty square next to an astronaut. On a phone, select **Place Flag** in your backpack.
- A flare illuminates a 5 × 5 grid of tiles for 11 seconds after landing. You can throw another one after 6 seconds.
- Hold the left mouse button on the LUR-1 wreckage, or select the wreckage and hold **Dig** on your phone. Repairing it costs 3 units of silicon and 7 units of regolith.
- The workshop allows you to upgrade your backpack from 35 kg to 110 kg. In Sandbox mode, you can add items to your backpack and disable quests in the mission creator.

## Launching



On this computer, double-click **Run-MoonBase101.cmd** in the `E:\giera` folder.

The script launches the pre-built MoonBase101.exe file. It does not require Rust to be installed.



On another computer with Rust and linker tools, in the `LunaGame` directory:



```powershell

cargo run --locked

```



## Controls



| Key | Action |

|---|---|

| WASD / arrow keys | Movement |

| Spacebar / Shift | Jump / jetpack in flight |

| Hold Left Mouse Button | Dig or demolish in pickaxe mode |

| 1–0, Left Mouse Button | Select module and build |

| B / Q | Toggle pickaxe / build mode |

| E | Enter / exit the base or rover |

| G at a station | Enter the cockpit or exit after landing |

| WASD / Spacebar / Ctrl in the cockpit | Fly horizontally / climb / land |

| N | Explorer’s log and planet selection in the cockpit |

| P | Examine a nearby purple research point |

| I / M / J / K | Backpack / Map / Progress / Character |

| R / T | Eat a meal / Drink water |

| C / X | Upgrade tool / First aid kit |

| Z / H | Change AI crew member / Flag |

| Scroll, + / − | Zoom in / Zoom out |

| O | Orbital radar |

| Esc | Close panel, pause, or resume |



You can enable on-screen buttons in the settings. You must

hold down the “ACTION” button to finish digging.



For your first expedition, you receive two rations and two portions of water. Additional supplies

are in the capsules. Enter a powered base and select “Make Water” to convert 1 ice

into 2 portions of water. “Make a Meal” consumes
1 portion of water and requires a connected
greenhouse. In Easy mode, the first base already has a greenhouse. In the event of a power outage,
preparation will pause and resume once power is restored. The spacesuit panel displays six gauges and estimates
the time remaining until the tank is empty. Digging, movement, and a leaky spacesuit
increase oxygen consumption. You can refill your tank at a powered base with a generator.

## Mining and the Geological Atlas

Deposits have a finite supply. The flat ore tile shrinks as you mine along its edge,
and an exhausted deposit disappears, leaving behind soil. Fragments drop after each cycle.
Walk up to them to collect resources and samples—mining alone does not add them
to your backpack. Debris remains in the world during the current expedition, even after
you leave the sector. You cannot collect it from inside the base or the rover.

Additional **Geological Atlas** objective: Collect ice, titanium, helium-3, and rare minerals,
then enter the base or lander. You’ll receive a one-time 120 GB report


for transmission. Progress (0–4) is visible in the top mission panel.

## Missions and Save

“Play” opens the “Report to Earth” mission briefing. Prepare water and food,
activate the antenna, explore four locations each on the Moon, Mars, and Venus,
and then send a report from the lunar base. The gold arrow indicates the current objective.
**P** starts a measurement at a marker or operates the antenna at the base.
Travel to distant points in the rover and exit before taking a measurement. The pace depends
on the route and stops. The mission is designed to last about 20 minutes, with no
countdown timer to failure.
“Design a Mission” lets you choose the difficulty, scenario, equipment, and character.

In solo mode (or as the host), enter the cockpit [G], open the log [N], and select Mars, Venus,
or a return to the Moon. Mars has dust storms, while Venus has extreme heat and a dense atmosphere. There are also
craters. Each world has four research points [P], even in custom mode. Buildings, resource deposits, vehicles, debris,
and discoveries persist during travel within a single expedition. Flight, weather,
and energy consumption are simplified for gameplay purposes.

Build modules side by side to connect their power supplies. The panel and generator
charge the batteries in the habitat and the warehouse. The oxygen generator operates on the
power grid. A powered laboratory analyzes samples, and the greenhouse prepares
ordered meals. The HUD inside displays power, oxygen, and battery
levels.

Materials for the journal: [Moon](https://science.nasa.gov/moon/facts/),
[Mars](https://science.nasa.gov/mars/facts/), [Venus](https://science.nasa.gov/venus/venus-facts/),
[gravity](https://nssdc.gsfc.nasa.gov/planetary/factsheet/),
[cosmic radiation](https://www.nasa.gov/hrp/hazard-space-radiation/).
Saving/loading `mission_design.json` applies to the **mission design**, not the world or
game progress. Returning from the settings to the pause screen preserves the current mission.
The main menu and switching modes end the current mission.

## LAN

In the main menu of the first computer, select “Host on PC · LAN and Phone.”
The host creates a server during gameplay and starts the mission. On the second computer,
open the settings, enter the host’s IPv4 address displayed under F10, and select “Join.”
The default port is TCP 7878. The two windows on a single computer connect via
127.0.0.1:7878. On your phone, open the web address displayed under F10 in your browser.
A maximum of four people can play in total. In Windows, select “Allow LAN”
from the main menu or under F10 and confirm the administrator’s permission. This rule applies to
the current EXE and TCP connections 7878 and 7879 from the local subnet.

The backpack [I] lets you drop a single item or an entire stack in open terrain.
Another player on the same planet can pick up the item. An open backpack
hides the quest tab. Outside the backpack, use the “Collapse Quests” button.

The host determines the weather, hazards, terrain conditions, deposits, structures, debris,
storage, measurements, and mission progress. The host approves digging, building, crafting,
picking up items, and shared emotes. An astronaut’s movement is local,
while other players see smoothed updates; spacesuit physiology is
calculated locally based on the host’s weather. Everyone has their own backpack and survival
statistics. Everyone travels separately to station [N]; the rest remain on their planet. Pressing Esc in the network opens the menu,
while the world simulation continues to run. The host logging out shuts down the server.

F5 — greeting, F6 — dance, F7 — joy. Approach a player and press F8
to invite them to high-five, or F9 to invite them to dance. The other player joins
using the same key within 8 seconds. The high-five has a shared animation and sound effect.

The mobile client is embedded in the game and accessible via the phone’s browser
on the same network. The host runs the page on TCP port 7879 during the session.

## Changes and Testing

- New expedition menu, consistent color palette, lighter typography, and UI scaling.
- Improved HUD clicking, panel hiding, zoom buttons, and returning from settings.
- Improved construction cost refunds, availability preview, and new mission reset.
- Limit on the number of distant objects rendered without losing map data.
- LAN feedback, visibility of joining players, and packet control.
- Fixed project read/write errors and normalized loaded data.

```powershell
cargo fmt --check
cargo test --locked
cargo clippy --locked
```

The original source code prior to the fixes is located in `.backup-original`.


Game authors: _wojtasz_, f57ers, hajaxu, melodyxmemorie, rafi2332, wrfghui.

## Expedition and TCP Update — November 14, 2026

- Backpack: 35 kg. Excess rewards fall to the ground. You can drop off
  cargo at the warehouse and retrieve it using the buttons at the base. The warehouse is shared across the session.
- Buildings cost about 50% more, tools about 20% more. All
  tool recipes fit within the backpack limit.
- Water requires an electrolyzer workstation; meals require a greenhouse. The airlock is not
  a production workstation. Power supply and ingredient reservation still apply.
- Rare deposits form procedural clusters. Random structures and chests
  appear much less frequently. Planets have distinct height generators,
  crater layouts, resources, and research locations. Returning to a planet preserves its state.
- After a threat, there are 90–150 seconds of calm (120–180 on Easy difficulty).
  Solar storms damage exposed spacesuits on all three worlds.
- A flare has a 6-second cooldown. The host enforces the same limit.
- Switching to Astronaut Z works on the ground outside the cabin; it retains equipment and
  avoids dangerous positions. Online, each player controls their own character.
- Tooltip descriptions, fading names of collected items, and
  a diagram of the eight planets of the Solar System with a bright Sun.

Tests: cargo test --locked. Build: cargo build --locked.

## Parallel Worlds and Passengers

The server maintains separate active worlds for the Moon, Mars, and Venus. At the station,
press N and select your own destination. Other players remain where they were.
The terrain, vehicles, production, warehouse, and weather belong to a specific planet;
the mission and discoveries are shared. When the host changes planets, the world
in which the others continue to play is preserved. Directional signs show the crew on your planet.

Pressing E next to the rover takes up a free seat: the first person drives, the others ride
as passengers. A light rover holds 2 people, a heavy one 4. Only the driver controls
WASD; E lets you get out in a safe place. After the driver exits, the first
passenger takes the wheel. You can see the number of occupied seats under the cursor.

## NASA Photo of the Day in the menu

The menu fetches APOD in a separate thread: title, date, type, description, preview, and an
HD link, if NASA provides one. The full description is scrolled using the scroll bar; the source text
remains in English. Videos open on the NASA website. No internet connection,
an API limit, or a NASA error does not prevent the game from starting. Refreshes are delayed by 30 seconds
to avoid repeated requests. The panel stores data in session memory.

The key is read from the NASA_API_KEY variable or the nasa_api_key.txt file located next to
the EXE file; DEMO_KEY is used by default. The private key file is not included in the
source archive. Windows uses the system-level HTTPS curl.exe and image decoder, without installing
additional libraries. The API may temporarily return an HTTP 500 error; in that case, a message is displayed.

The latest Windows build passed 43 tests. The test using current NASA

APOD is skipped by default and requires a connection to NASA.

When the APOD API returns the general NASA logo or an error, the game retrieves the current article
from the public website science.nasa.gov/apod/, selecting an image from within the article.
The title, date, description, and author of the image remain associated with the same discovery.
