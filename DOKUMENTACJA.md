# Moon Base 101

Dokumentacja gry i kodu źródłowego · `moon-base-101` 0.1.0 · 14 listopada 2026

## 1. Czym jest gra

**Moon Base 101** to edukacyjna gra survivalowa 2.5D z widokiem izometrycznym. Gracz dowodzi wyprawą na Księżyc, Marsa i Wenus. Zbiera zasoby, utrzymuje skafander i bazę, korzysta z łazika i stacji latającej oraz prowadzi pomiary w terenie. Gra ma tryb solo i kooperację w sieci lokalnej dla maksymalnie czterech osób łącznie z hostem.

W kampanii **Raport dla Ziemi** gracz najpierw przygotowuje wyprawę, potem bada **12 miejsc**, po cztery na każdym świecie, i przesyła raport z bazy na Księżycu. Projekt zakłada około **20 minut** rozgrywki. Tempo zależy od trasy, zapasów i postojów, a gracz może kontynuować po upływie 20 minut.

**Zaprojektuj misję** to tryb z własnym scenariuszem. Pozwala dobrać trudność, załogę, rakietę, instrumenty naukowe, zasilanie, łączność i wyposażenie. Każdy z trzech światów udostępnia cztery punkty badań także w tym trybie.

## 2. Silnik i technologie

| Element | Technologia i zastosowanie |
|---|---|
| Silnik | **Bevy 0.13.2**. Systemy ECS, renderowanie 2D, obsługa zasobów, interfejs i narzędzia rysunkowe Gizmos. |
| Język | **Rust**, edycja 2021. Cargo zarządza budową, uruchamianiem i testami. Wersje zależności utrwala `Cargo.lock`. |
| Grafika | Sprite’y i kamera 2D Bevy. Funkcje gry rzutują współrzędne terenu `(x, y, z)` na ekran i wyznaczają kolejność rysowania obiektów. |
| Okno i wejście | `winit 0.29.15` i Bevy. Obsługa klawiatury, myszy i ekranowych przycisków sterowania. |
| Dźwięk | `rodio 0.17.3`, m.in. efekty narzędzi, interfejsu, flar i emotek. |
| Generowanie | `rand 0.8.8` oraz własne algorytmy terenu, złóż i zdarzeń. |
| Dane | `serde 1.0.229` i `serde_json 1.0.154`. JSON dla komunikatów gry, projektu misji i odpowiedzi APOD. |
| Grafika WWW | `image 0.24.9` koduje atlas PNG z proceduralnych tekstur Bevy. Przeglądarka pobiera atlas i jego opis z hosta. |
| Gra sieciowa na PC | Gniazda TCP biblioteki standardowej Rust i własny protokół gry. |
| Telefon | `tiny_http 0.12.0` po stronie hosta. Klient `web/mobile.html` korzysta z HTML, CSS, JavaScript i Canvas 2D. |
| Adres hosta | `if-addrs 0.13.4` pomaga wykrywać adresy interfejsów sieciowych pokazywane pod F10. |
| Zapora Windows | PowerShell i polecenia Windows Firewall. Biblioteka `base64 0.21.7` koduje argumenty PowerShell dla poprawnej obsługi ścieżek z odstępami i polskimi znakami. |
| Typografia | Osadzony Technor Variable z polskimi znakami, używany przez aplikację i klienta WWW. |

W Bevy encja jest identyfikatorem obiektu, komponent przechowuje jego stan, a system aktualizuje obiekty z określonymi komponentami. Astronauta ma np. komponent pozycji i sterowania, a system fizyki obsługuje jego ruch i kolizje. Dane sesji są przechowywane również jako zasoby ECS, np. `LunarMap`, `SurvivalStats`, `MissionConfig` i `LanMultiplayer`. `AppState` steruje przejściami między menu, odprawą, powierzchnią, pauzą i raportem.

## 3. Przebieg kampanii

| Etap | Co zrobić |
|---|---|
| 1. Zapasy | Podejdź do wskazanej skrzyni i otwórz ją. |
| 2. Woda | W zasilonej bazie podejdź do elektrolizera i uruchom produkcję. |
| 3. Posiłek | Przy zasilonej szklarni przygotuj jedzenie. |
| 4. Łączność | Uruchom antenę bazy. |
| 5. Księżyc | Zbadaj cztery oznaczone miejsca. |
| 6. Mars | Poleć na Marsa i wykonaj cztery pomiary. |
| 7. Wenus | Poleć na Wenus i wykonaj cztery pomiary. |
| 8. Raport | Wróć na Księżyc i prześlij raport przez zasiloną antenę. |

Złota strzałka wskazuje aktualny cel. Przy fioletowym znaczniku wciśnij **P**, żeby uruchomić pomiar, i pozostań w zasięgu. Kod kampanii przewiduje **25 s na pomiar** i **18 s na końcową transmisję**. Odkrycia odblokowują krótkie informacje o badanych miejscach. Karta zadań ma przycisk zwijania i chowa się na czas otwarcia plecaka.

## 4. Światy i część edukacyjna

| Świat | Grawitacja w grze | Warunki w grze | Tematy czterech badań |
|---|---:|---|---|
| Księżyc | 1,62 m/s² | Mała grawitacja i prawie próżnia. Oświetlenie i temperatura zależą od pory. | Ślad uderzenia, regolit, cień krateru, ślady misji. |
| Mars | 3,73 m/s² | Rzadka atmosfera z CO₂, mroźne otoczenie i burze pyłowe ograniczające produkcję paneli. | Rdzawy pył, zamarznięta woda, pole pyłu, ślad dawnej wody. |
| Wenus | 8,87 m/s² | Gęsta atmosfera z CO₂ i temperatura otoczenia ustawiona w grze na 467 °C. Skafander zużywa energię na ochronę termiczną. | Skały wulkaniczne, gęsta atmosfera, mapa radarowa, gorąca równina. |

Każdy świat używa odrębnego ziarna proceduralnego, ukształtowania powierzchni, układu kraterów, rozmieszczenia zasobów i współrzędnych badań. Punkty badań są zdefiniowane w kodzie, a teren i skupiska rud są generowane sektorami. Gra może usunąć odległe sprite’y z renderowania, zachowując dane o terenie. Po powrocie na poprzedni świat w tej samej wyprawie zachowują się m.in. zmiany złóż, budowle i pojazdy.

Grawitacja wpływa na skoki i spadanie, a biomy różnią się oddziaływaniem na skafander i bazę. Lot między światami, środowisko, dawka promieniowania i bilans energii mają uproszczenia rozgrywkowe. Odczyty skafandra są wskaźnikami gry. Przykładowe tematy do omówienia po misji to różnica między masą a ciężarem, wpływ pyłu na panele słoneczne i mapowanie powierzchni Wenus przez chmury za pomocą radaru.

## 5. Przetrwanie, wydobycie i baza

**Skafander.** HUD pokazuje m.in. stan zdrowia, tlen, zapasy organizmu, energię i szczelność. Temperatura ciała wpływa na przeżycie. Marsz i kopanie zwiększają zużycie tlenu, a rozszczelnienie zwiększa straty na zewnątrz. Śluza umożliwia przejście do bazy. Aby habitat uzupełniał tlen w skafandrze, jego sieć potrzebuje działającego generatora tlenu i zasilania. Kabina stacji i ciśnieniowy łazik również mogą uzupełniać tlen.

**Zagrożenia.** Zależnie od świata występują burze pyłowe i słoneczne, meteoryty, wstrząsy gruntu oraz epizody zimna. Burza słoneczna może uszkadzać odsłonięty skafander na wszystkich trzech światach. W zdarzeniach znaczenie ma schronienie i stan pojazdu lub bazy. Między zdarzeniami występują okresy spokoju. Flara ma 6 s odnowienia. Apteczka zużywa jedną sztukę i odnawia do 50 punktów zdrowia. Dawka promieniowania pozostaje zapisana.

**Złoża i plecak.** Kafelek rudy stopniowo maleje w trakcie kopania, a złoże znika po wyczerpaniu. Wydobyty surowiec wypada na ziemię. Żeby go podnieść, podejdź pieszo. Plecak ma limit **35 kg**. Jeśli w plecaku zabraknie miejsca, przedmioty pozostają na ziemi do późniejszego zebrania. Na otwartym terenie można wyrzucić jedną sztukę lub cały stos przyciskami w plecaku. Inny gracz na tym samym świecie może podnieść wyrzucony przedmiot. Po zebraniu pojawia się zanikająca etykieta.

**Budowanie.** Moduły stojące przy sobie bokami tworzą sieć. Panele słoneczne zależą od światła, a RTG zasila sieć również w ciemności. Habitat i magazyn przechowują energię w akumulatorach. Warsztat i pasek budowy pokazują koszty dostępnych receptur. W grze można postawić dziesięć typów modułów.

| Moduł | Zastosowanie |
|---|---|
| Śluza | Wejście i wyjście z połączonej bazy. |
| Habitat | Schronienie, magazynowanie energii i dostęp do tlenu z podłączonego generatora. |
| Panel słoneczny | Produkcja prądu zależna od światła i warunków na powierzchni. |
| Generator RTG | Źródło prądu niezależne od oświetlenia. |
| Elektrolizer | Umożliwia zasilonej bazie uzupełnianie tlenu. Przy stanowisku gracz zamawia 2 porcje wody z 1 jednostki lodu. |
| Szklarnia | Przygotowuje posiłek z 1 porcji wody. |
| Magazyn | Przyjmuje i wydaje ładunek. W LAN gracz korzysta ze wspólnego magazynu danego świata. |
| Antena | Umożliwia przesył danych i końcowego raportu. |
| Laboratorium | Analizuje próbki w zasilonej bazie. |
| Mur osłonowy | Chroni sąsiednie moduły przed meteorytami. |

Produkcja wody i jedzenia rezerwuje składniki po przyjęciu zlecenia. Woda powstaje w 4 s, a posiłek w 12 s pracy stanowiska. Utrata zasilania zatrzymuje postęp, który można wznowić po przywróceniu prądu.

**Pojazdy.** Łazik ułatwia dotarcie do odległych punktów badań. Lekki mieści dwie osoby, ciężki cztery. Pierwszy gracz prowadzi, kolejni zajmują miejsca pasażerów. Po wyjściu kierowcy kolejny pasażer może przejąć prowadzenie. Osobno można pilotować stację nad powierzchnią i użyć dziennika przy stacji, aby zmienić świat.

## 6. Sterowanie na PC

Na górze **Ustawień** przycisk **Pełny ekran** pokazuje bieżący stan i przełącza między pełnym ekranem bez ramek i trybem okienkowym. Skrót **F11** działa również podczas misji. Powrót do okna przywraca jego poprzedni rozmiar i pozycję.

| Sterowanie | Działanie |
|---|---|
| `WASD` lub strzałki, `Spacja`, `Shift` | Ruch, skok, plecak odrzutowy w powietrzu. |
| Przytrzymaj lewy przycisk myszy | Kopanie złoża lub rozbiórka w trybie kilofa. |
| `B` / `Q`, `1–0` | Zmiana trybu kopania i budowania, wybór modułu. |
| `E` | Wejście lub wyjście z bazy albo łazika. |
| `G`, `WASD`, `Spacja`, `Ctrl` | Wejście do kokpitu przy stacji, lot poziomy, wznoszenie, lądowanie. Po lądowaniu `G` pozwala wysiąść. |
| `N`, `P` | Dziennik i wybór świata, rozpoczęcie badania lub obsługa celu misji. |
| `I`, `M`, `J`, `K` | Plecak, mapa ze znacznikami, postęp, wygląd astronauty. |
| `R`, `T`, `C`, `F`, `X` | Jedzenie, picie, ulepszenie narzędzia, naprawa, apteczka. |
| `H`, `Z` | Postawienie flagi ze znacznikiem, przełączenie astronauty AI w solo. |
| `F5`–`F9` | Powitanie, taniec, radość, piątka i wspólny taniec. Wspólna emotka wymaga przyjęcia zaproszenia w pobliżu w ciągu 8 s. |
| `F10`, `Esc` | Panel połączenia hosta, zamknięcie panelu lub pauza i wznowienie. W LAN otwarcie pauzy nie zatrzymuje serwera. |

## 7. Multiplayer i telefon

1. Na komputerze z Windows wybierz **Zezwól na LAN** i potwierdź zgodę administratora. Przycisk jest w menu głównym, ustawieniach i panelu F10. Reguła zapory obejmuje bieżący plik EXE oraz ruch z `LocalSubnet` na TCP **7878** i **7879**. Po przeniesieniu EXE do innego folderu zastosuj regułę ponownie.
2. Wybierz **Hostuj na PC · LAN i telefon**. Serwer uruchamia się w procesie gospodarza.
3. Na drugim komputerze wpisz adres z panelu F10, np. `192.168.1.10:7878`, w ustawieniach i kliknij **Dołącz**. Do testu na tym samym komputerze służy `127.0.0.1:7878`.
4. Na telefonie w tej samej sieci otwórz adres WWW z F10, np. `http://192.168.1.10:7879/`. Wpisz nazwę i dołącz. Ten adres można wykorzystać we własnym kodzie QR. Telefon obsługuje wirtualny joystick, mapę, kopanie i akcje z menu **Plecak i akcje**.

Host rozstrzyga pogodę, zmiany terenu, budowę, wydobycie, podnoszenie i wyrzucanie ładunku, magazyn, badania i wspólne emotki. Postęp misji jest wspólny, a każdy gracz ma własny plecak. Teren, zasilanie, magazyny, pojazdy i zdarzenia są przypisane do świata. Podróż jednego gracza nie przenosi pozostałych. Serwer aktualizuje również światy odwiedzane przez graczy przebywających poza planetą hosta. Zakończenie sesji przez hosta rozłącza uczestników i zatrzymuje stronę mobilną.

**Synchronizacja PC.** `net_tcp.rs` korzysta z nieblokujących gniazd TCP. Ramka zaczyna się od 4-bajtowej długości w porządku big-endian, po której następuje JSON. Moduł ogranicza rozmiar pojedynczej ramki do 256 KiB i kolejkę wyjściową do 2 MiB. Klient desktopowy przewiduje własny ruch lokalnie, a pozycje innych astronautów interpoluje z buforem około 100 ms. Host zatwierdza działania wpływające na wspólny stan i wysyła aktualizacje świata, w tym zmienione kafelki i pozycje obiektów.

**Transport WWW.** `GET /` przekierowuje do `/play/<token>` aktualnej sesji. Przeglądarka pobiera osadzoną w EXE stronę i nawiązuje stałe połączenie WebSocket pod `/play/<token>/socket`. Host przesyła stan graczy oraz zmienione fragmenty terenu i nowe linie czatu. Przeglądarka wygładza ruch między odczytami stanu. Klient WWW i klient natywny korzystają z tej samej symulacji hosta.

## 8. NASA, zapis i uruchomienie

**Czat i lista graczy.** Przytrzymany Tab pokazuje nazwę gracza i ping do hosta. Enter otwiera wspólny czat aplikacji, a klienci WWW korzystają z przycisku Czat. Wiadomości docierają między światami. Host rozsyła historię nowym klientom i ogranicza wiadomość do 160 znaków. Utrata połączenia z hostem kończy rozgrywkę klienta. Przeglądarka wraca do ekranu dołączania po wykryciu niedostępności hosta.

**Wygląd i grafika WWW.** Sieć przesyła kolor skafandra, wizjera, dodatki hełmu, kolor smugi i aktywność napędu. Klient WWW pobiera atlas tych samych tekstur, z których korzysta aplikacja Bevy. Obie prezentacje używają proporcji rzutu izometrycznego 88 × 44 i skali wysokości 28. Strona rysuje również dachy, łączniki modułów i cienie, a HUD jest dostosowany do dotyku. Renderowanie WWW obsługuje Canvas 2D. Identyfikatory kart przeglądarki są rozdzielane, a miejsca startowe graczy WWW są wybierane w odstępie od siebie.

**Rozbiórka.** Przycisk na górnym pasku aplikacji przełącza tryb rozbiórki. Przytrzymanie LPM na budynku usuwa go i pozostawia materiały odpowiadające zapisanym kosztom budowy. Telefon obsługuje analogiczny tryb przytrzymanej akcji. O usunięciu budynku i przedmiotach rozstrzyga host.

**NASA APOD.** Panel **Odkryj kosmos · NASA** pobiera zdjęcie lub odnośnik do filmu, tytuł, datę i opis w wątku roboczym. Opis źródłowy jest po angielsku. Gra może też odczytać bieżący publiczny artykuł APOD. Klucz odczytuje ze zmiennej `NASA_API_KEY` lub z `nasa_api_key.txt` obok EXE. Domyślnie używa `DEMO_KEY`. Prywatny plik klucza nie jest składnikiem paczki źródeł. Brak dostępu do NASA nie blokuje rozgrywki. Obecny kod podglądu zdjęć APOD korzysta z dekodera Windows.

**Zapis.** Plik `mission_design.json` przechowuje projekt misji, czyli wybrane ustawienia z trybu projektowania. Stan powierzchni, badania i ekwipunek żyją w pamięci bieżącej wyprawy. Menu pauzy pozwala wrócić do tej samej sesji. Rozgrywkę po zamknięciu procesu zaczyna się od początku.

**Gotowa gra na Windows.** Rozpakuj całą paczkę, a następnie uruchom `MoonBase101.exe` albo `Uruchom-MoonBase101.cmd`. Gotowy plik nie wymaga instalacji Rusta.

**Budowa ze źródeł.** Potrzebujesz Rusta, Cargo, linkera i bibliotek systemowych grafiki oraz dźwięku. Na Linuksie wybierz osobny wariant X11 z Vulkanem albo Wayland z OpenGL za pomocą `sh run.sh --x11` lub `sh run.sh --wayland`. Oba warianty korzystają z tej samej symulacji i protokołu sieciowego co Windows. Instalowanie gotowych paczek i połączenie z hostem opisuje `LINUX.md`.

```sh
cargo run --locked
cargo build --release --locked
cargo fmt --check
cargo test --locked
```

Testy sieciowe wymagają możliwości otwierania lokalnych gniazd. Build ORBIT-013 na Windows zaliczył **46 testów**, a jeden test NASA wymagający internetu i dekodera systemowego pominięto. Testy obejmują m.in. wsiadanie hosta klawiszem E i przejęcie kierownicy przez pasażera, sesję WebSocket, koszt naprawy LUR-1, zasięg i czas światła flary, kampanię oraz lot i lądowanie. Test fizycznego połączenia między komputerami i uruchomienie na Linuksie pozostają do weryfikacji.

## 9. Mapa kodu

| Plik lub grupa | Odpowiedzialność |
|---|---|
| `src/main.rs`, `src/types.rs` | Uruchomienie Bevy, zarejestrowanie ekranów i systemów, konfiguracja i typy gry. |
| `src/ui_screens.rs`, `src/hover.rs` | Menu, HUD, questy, plecak, mapa, pauza, ustawienia, opisy pod kursorem. |
| `src/world.rs`, `src/planets.rs`, `src/dimensions.rs` | Generowanie sektorów, definicje światów, przejścia i symulacja światów poza planetą hosta. |
| `src/isometric.rs`, `src/celestial.rs` | Rzutowanie, proceduralne zasoby graficzne, oświetlenie, tło nieba i schemat Układu Słonecznego. |
| `src/physics_gameplay.rs`, `src/mineral.rs`, `src/base.rs` | Ruch i przeżycie, zdarzenia, kopanie, przedmioty, sieć bazy i produkcja. |
| `src/campaign.rs`, `src/flight.rs`, `src/vehicles.rs` | Etapy misji, lot stacji, pojazdy i miejsca dla pasażerów. |
| `src/net_tcp.rs`, `src/net_lan.rs`, `src/net_world.rs`, `src/net_visuals.rs`, `src/emotes.rs` | Transport TCP, sesja, autorytatywne akcje i stan, prezentacja innych graczy i emotki. |
| `src/mobile_host.rs`, `src/mobile_player.rs`, `web/mobile.html` | Wbudowane HTTP, gracze mobilni i dotykowy klient WWW. |
| `src/firewall.rs`, `src/nasa_apod.rs`, `src/audio.rs` | Zapora Windows, integracja z NASA i dźwięk. |

Autorzy wskazani w grze · _wojtasz_, f57ers, hajaxu, melodyxmemorie, rafi2332, wrfghui.
