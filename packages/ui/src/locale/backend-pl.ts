// Polish wording of coded backend messages and vocabulary terms.
// Generated from the Rust inventory (scripts/backend-messages.mjs); edit the text here.
// English is the backend's own text and is never duplicated in this file.

/** Message and term codes with their Polish wording. */
export const backendPl: Readonly<Record<string, string>> = {
  "check.ai.differs": "{scaling}: różnica {deviation} ml",
  "check.ai.fourWire": "4-przewodowy, wejście pasywne",
  "check.ai.noReading": "{label} na {channel}: brak odczytu",
  "check.ai.noSignal":
    "{label}: brak prawidłowego sygnału do porównania ze wziernikiem",
  "check.ai.none": "Ta maszyna nie ma przetworników analogowych.",
  "check.ai.raw":
    "{label} na {channel}: {measured} {unit}, wartość surowa {raw} ({quality})",
  "check.ai.scaling":
    "{label}: {percent} % zakresu {min}..{max} ml = {value} ml; wziernik {level} ml",
  "check.ai.separate": "z oddzielnym zasilaniem",
  "check.ai.signalOk": "{label}: {signal} {power}; {channel} jest zgodny",
  "check.ai.twoWire": "2-przewodowy, pętla zasilana z wejścia",
  "check.diag.ConnectionRefused": "połączenie odrzucone",
  "check.diag.ConnectionTimeout": "przekroczenie czasu połączenia",
  "check.diag.FirstScan": "wykonano pierwszy cykl",
  "check.diag.ForceActive": "aktywne wymuszenie",
  "check.diag.HardwareDownloaded": "wgrano konfigurację sprzętu",
  "check.diag.IpAddressConflict": "konflikt adresów IP",
  "check.diag.LinkDown": "brak łącza sieciowego",
  "check.diag.MemoryReset": "reset pamięci",
  "check.diag.ModbusException": "wyjątek Modbus {code}",
  "check.diag.ModeChange": "zmiana stanu pracy",
  "check.diag.ModuleMismatch": "niewłaściwy typ modułu (zamontowany: {name})",
  "check.diag.ModuleMissing": "brak modułu",
  "check.diag.NoFieldPower": "brak zasilania obiektowego (obciążenia)",
  "check.diag.NoRoute": "brak trasy do urządzenia",
  "check.diag.Overflow": "przepełnienie",
  "check.diag.PlcFault": "usterka PLC",
  "check.diag.ProgramDownloaded": "wgrano program",
  "check.diag.Startup": "uruchomiono CPU",
  "check.diag.UnconfiguredModule": "modułu nie ma w projekcie",
  "check.diag.Underflow": "niedomiar",
  "check.diag.WireBreak": "przerwanie przewodu",
  "check.drive.absent": "nieobecne",
  "check.drive.command": "{label}: polecenie pracy {state}",
  "check.drive.dark": "{label}: brak wskazań, przemiennik nie ma zasilania",
  "check.drive.enable": "{label}: zacisk zezwolenia na pracę {state}",
  "check.drive.energized": "pod napięciem",
  "check.drive.faulted": "usterka",
  "check.drive.keypad": "panelu",
  "check.drive.local":
    "{label}: sterowanie lokalne, polecenie pracy z PLC jest ignorowane",
  "check.drive.noEnable": "brak zezwolenia na pracę",
  "check.drive.noEnableTerminal": "{label}: brak zacisku zezwolenia na pracę",
  "check.drive.noFault": "{label}: brak usterki",
  "check.drive.noSupply": "brak zasilania sieciowego",
  "check.drive.notEnergized": "bez napięcia",
  "check.drive.notReady": "{label}: niegotowy ({reasons})",
  "check.drive.notReadyCommanded":
    "{label}: niegotowy ({reasons}); polecenie pracy jest aktywne",
  "check.drive.present": "obecne",
  "check.drive.ready": "{label}: gotowy",
  "check.drive.reference": "{label}: wartość zadana {percent} % z {source}",
  "check.drive.remote": "{label}: zdalne (podąża za PLC)",
  "check.drive.running": "pracuje",
  "check.drive.speed": "{label}: {state}, prędkość {percent} %",
  "check.drive.stopped": "zatrzymany",
  "check.drive.terminals": "zacisków",
  "check.drive.tripped": "{label}: usterka (przekaźnik usterki zamknięty)",
  "check.how.actuator":
    "Widok Proces: czy urządzenie porusza się, gdy przełącza się jego wyjście?",
  "check.how.address": "Konfiguracja sieci: interfejs PLC.",
  "check.how.addressConflict": "Widok Sieć: adresy urządzeń.",
  "check.how.addressMissing": "Konfiguracja sieci: ustaw adres interfejsu PLC.",
  "check.how.alarms": "Widok Proces i Diagnostyka: stany alarmowe.",
  "check.how.analogRaw":
    "Zmierz prąd pętli lub napięcie na zacisku; tabela podglądu (Watch): wartość surowa.",
  "check.how.analogSignal":
    "Widok Sprzęt (tryb ZAAWANSOWANY): tryb kanału i zasilanie pętli w porównaniu ze schematem.",
  "check.how.commPower":
    "Widok Sieć: stan urządzeń; wykonaj ping do urządzenia.",
  "check.how.compare":
    "Widok Sprzęt (Porównanie): moduły skonfigurowane w porównaniu z wykrytymi.",
  "check.how.configuration": "Widok Sprzęt: adresy i przypisanie sygnałów.",
  "check.how.contactType":
    "Porównaj wejście w spoczynku ze schematem: NC ma w spoczynku TRUE, NO FALSE.",
  "check.how.controller": "Widok Sprzęt: diody stanu CPU.",
  "check.how.cpu": "Widok Sprzęt: porównaj projekt z zainstalowanym CPU.",
  "check.how.cyclic":
    "Widok Sieć: stan połączeń; PLC odpytuje tylko w trybie RUN.",
  "check.how.devices":
    "Widok Sieć: urządzenia; wykonaj ping do urządzenia po jego adresie.",
  "check.how.diState":
    "Widok Sprzęt: diody wejść; tabela podglądu (Watch): wartości wejść i ich jakość.",
  "check.how.diagnostics": "Widok Diagnostyka: bufor diagnostyczny.",
  "check.how.driveSupply":
    "Wyświetlacz przemiennika; zasilanie sieciowe mierz tylko kwalifikowanym, bezpiecznym sprzętem.",
  "check.how.feedback":
    "Tabela podglądu (Watch): wejście sygnału zwrotnego, gdy element wykonawczy jest zasilony.",
  "check.how.fieldSupply": "Widok Sprzęt: stan modułów wyjść.",
  "check.how.interlocks":
    "Widok Proces: przekaźniki przeciążeniowe i urządzenia zabezpieczające.",
  "check.how.ioSupply":
    "Widok Sprzęt: stan modułów; zmierz zasilanie 24 V modułu.",
  "check.how.link": "Widok Sieć: diody łącza; sprawdź kabel.",
  "check.how.logical": "Załącz każde wyjście z programu lub wymuszeniem.",
  "check.how.modulePower":
    "Widok Sprzęt: diody stanu modułów; moduł bez zasilania elektroniki nie odpowiada.",
  "check.how.moduleStatus": "Widok Sprzęt: diody stanu modułów.",
  "check.how.permissives": "Widok Proces: poziomy i wyłączniki krańcowe.",
  "check.how.physical":
    "Widok Sprzęt: dioda wyjścia; zmierz napięcie zacisku względem M.",
  "check.how.plcFault":
    "Widok Diagnostyka: usterka PLC; po jej usunięciu zresetuj PLC.",
  "check.how.plcRun": "Pasek narzędzi: tryb PLC i program.",
  "check.how.plcStop":
    "Pasek narzędzi: po wgraniu programu (Kompiluj i wgraj do PLC) przełącz PLC w tryb RUN.",
  "check.how.quality": "Widok Sieć: liczniki zapytań i błędów.",
  "check.how.scaling":
    "Porównaj wartość przetwornika ze wziernikiem, a potem ze swoją przeskalowaną wartością w tabeli podglądu (Watch).",
  "check.how.sensorOperation":
    "Uruchamiaj każde urządzenie w widoku Proces i obserwuj, czy jego wejście za nim podąża.",
  "check.how.sequence":
    "Widok Program: wartości online; Przyczyna i skutek: ostatni cykl.",
  "check.how.timeout":
    "Tabela podglądu (Watch): wejścia sygnałów zwrotnych po poleceniu.",
  "check.hw.connection": "połączenie {index}",
  "check.hw.consistent":
    "Brak nakładających się adresów; każde przypisanie wskazuje użyteczny kanał",
  "check.hw.cpuDiffers": "w projekcie {configured}, zainstalowany {installed}",
  "check.hw.cpuMatches": "{cpu} zainstalowany i skonfigurowany",
  "check.hw.diagnostic": "{at}: {diagnostic}",
  "check.hw.interface": "interfejs PLC",
  "check.hw.led": "{slot} {name}: dioda stanu {led}",
  "check.hw.ledsGreen": "Wszystkie diody stanu modułów świecą na zielono",
  "check.hw.noDiagnostic": "Brak aktywnej diagnostyki",
  "check.hw.slotsMatch": "Pozycje modułów zgodne z konfiguracją: {count}",
  "check.hw.typesMatch": "Każdy zainstalowany moduł ma skonfigurowany typ",
  "check.in.contactDiffers":
    "{label}: na schemacie {drawing}, ale {drawn} ma w spoczynku {value}",
  "check.in.contactOk": "{label}: na schemacie {drawing}, w spoczynku {value}",
  "check.in.contactUntested":
    "{label}: na schemacie {drawing}; uruchom urządzenie, aby porównać",
  "check.in.follows": "{label}: {drawn} podąża za urządzeniem",
  "check.in.nc": "normalnie zamknięty (TRUE w spoczynku)",
  "check.in.no": "normalnie otwarty (FALSE w spoczynku)",
  "check.in.noDevices":
    "Do wejść nie podłączono urządzeń obsługi ani czujników.",
  "check.in.noInput":
    "{label}: uruchomienie nie zmieniło żadnego wejścia; {drawn} za nim nie podążał",
  "check.in.notOperated":
    "{label}: jeszcze nie uruchomiony (na schemacie: {drawn})",
  "check.in.otherInput":
    "{label}: uruchomienie zmieniło {changed} zamiast {drawn}",
  "check.in.quality": "{channel}: {quality}",
  "check.in.value": "{channel} = {value}",
  "check.more": "... i jeszcze {count}",
  "check.net.conflict": "Adresu {ip} używa inne urządzenie",
  "check.net.device": "{label} pod adresem {ip}: {state}",
  "check.net.deviceNoLink": "{label}: brak łącza",
  "check.net.errors":
    "{id} ({server}): przekroczenia czasu: {timeouts}, wyjątki: {exceptions}, zapytania: {requests}",
  "check.net.interface": "Interfejs PLC {ip} / {mask}",
  "check.net.linksUp": "Wszystkie porty mają łącze",
  "check.net.noAddress": "Interfejs PLC nie ma adresu IP",
  "check.net.noErrors": "Brak przekroczeń czasu i wyjątków",
  "check.net.none": "Ta maszyna nie ma sieci.",
  "check.net.notResponding": "nie odpowiada",
  "check.net.online": "online",
  "check.net.plcNoLink": "Port PLC: brak łącza",
  "check.net.scan": "{id} -> {server}: {status}",
  "check.noDrive": "Ta maszyna nie ma przemiennika.",
  "check.out.dead": "{channel}: polecenie ON, ale zacisk pozostał na 0 V",
  "check.out.energizedOff":
    "{label}: zasilony, choć {channel} był wyłączony (OFF)",
  "check.out.feedbackFollows": "{label}: sygnał zwrotny {feedback} podąża",
  "check.out.feedbackMissing":
    "{label}: zasilany przez {seconds} s, sygnał zwrotny {feedback} ani razu się nie zmienił",
  "check.out.feedbackUntested":
    "{label}: zasil go, aby zobaczyć sygnał zwrotny",
  "check.out.feedbackWait":
    "{label}: zasil go na kilka sekund, aby zobaczyć {feedback}",
  "check.out.live": "{channel}: zacisk przełącza 24 V",
  "check.out.logical": "{channel} -> {label}: {state}",
  "check.out.noActuators": "Brak podłączonych elementów wykonawczych.",
  "check.out.noFeedback": "Brak elementów wykonawczych z sygnałem zwrotnym.",
  "check.out.noModules": "Brak modułów wyjść.",
  "check.out.noOutputs": "Brak podłączonych wyjść.",
  "check.out.noResponse":
    "{channel} przełączył 24 V, ale {label} nie zareagował",
  "check.out.notEnergized": "{label}: jeszcze nie zasilony",
  "check.out.notSwitched": "{channel}: jeszcze nie załączony (ON)",
  "check.out.off": "WYŁ.",
  "check.out.on": "WŁ.",
  "check.out.responds": "{label}: reaguje na {channel}",
  "check.out.supplied": "Moduły wyjść z zasilaniem obciążenia: {count}",
  "check.plc.fault": "FAULT: {detail}",
  "check.plc.faultEmpty": "FAULT",
  "check.plc.run": "RUN, program {name} (wgranie {revision})",
  "check.plc.runEmpty": "RUN",
  "check.plc.stopEmpty": "STOP: nie wgrano programu",
  "check.plc.stopLoaded": "STOP: program jest wgrany, ale nie jest wykonywany",
  "check.power.controller": "{cpu} zasilony",
  "check.power.deviceSilent": "{label} ({ip}) nie odpowiada",
  "check.power.devicesOnline": "Zasilone urządzenia sieciowe online: {count}",
  "check.power.driveSupplied": "{label}: zasilanie sieciowe załączone",
  "check.power.driveUnsupplied":
    "{label}: brak zasilania sieciowego (wyświetlacz ciemny, niegotowy)",
  "check.power.fieldSupplied": "Moduły we/wy z zasilaniem obiektowym: {count}",
  "check.power.modulesAnswer": "Każdy skonfigurowany moduł odpowiada CPU",
  "check.power.noNetworkDevices": "Ta maszyna nie ma urządzeń sieciowych.",
  "check.process.dry": "{label}: praca na sucho",
  "check.process.highHigh": "powyżej poziomu bardzo wysokiego",
  "check.process.limit": "{label}: {limit}",
  "check.process.lowLow": "poniżej poziomu bardzo niskiego",
  "check.process.noAlarms": "Brak stanu alarmowego procesu",
  "check.process.noTimeouts":
    "Żaden element wykonawczy nie czeka na sygnał zwrotny",
  "check.process.noTrips": "Żadne urządzenie zabezpieczające nie zadziałało",
  "check.process.notRunning":
    "PLC nie wykonuje programu (nie jest w trybie RUN)",
  "check.process.overflowing": "przelewa się",
  "check.process.overload": "{label}: zadziałał przekaźnik przeciążeniowy",
  "check.process.running": "PLC wykonuje swój program (cykle: {scans})",
  "check.process.timeout":
    "{channel}: zasilony bez oczekiwanego sygnału zwrotnego",
  "check.process.withinLimits": "Proces mieści się w swoich granicach",
  "check.scan.exception": "wyjątek {code}",
  "check.scan.idle": "bezczynne (PLC nie odpytuje)",
  "check.scan.noInterface": "brak interfejsu PLC",
  "check.scan.noRoute": "brak trasy",
  "check.scan.ok": "wymiana danych (odpowiedzi: {count})",
  "check.scan.refused": "połączenie odrzucone",
  "check.scan.timeout": "przekroczenie czasu",
  "device.actuated": "uruchomione",
  "device.closed": "styk zamknięty",
  "device.energized": "pod napięciem",
  "device.noSignal": "brak sygnału",
  "device.notActuated": "nieuruchomione",
  "device.notEnergized": "bez napięcia",
  "device.open": "styk otwarty",
  "device.sendsMa": "wysyła {value} mA",
  "device.sendsV": "wysyła {value} V",
  "device.unknown": "nieznany",
  "diag.value.none": "brak",
  "event.alarm": "{alarm} ({what})",
  "event.alarm.acknowledged": "potwierdzony",
  "event.alarm.activated": "alarm",
  "event.alarm.returned": "powrót do normy",
  "event.alarm.withdrawn": "wycofany",
  "event.coldStart": "Start zimny: wszystkie dane programu zainicjalizowane",
  "event.diag.ConnectionRefused":
    "Zdalne urządzenie odrzuciło połączenie (pod tym adresem nie ma serwera Modbus).",
  "event.diag.ConnectionTimeout":
    "Brak odpowiedzi zdalnego urządzenia w wyznaczonym limicie czasu.",
  "event.diag.FirstScan.cold":
    "Wykonano pierwszy cykl programu po restarcie zimnym.",
  "event.diag.FirstScan.warm":
    "Wykonano pierwszy cykl programu po restarcie ciepłym.",
  "event.diag.ForceActive": "Aktywne wymuszenie: wartości we/wy są nadpisane.",
  "event.diag.HardwareDownloaded":
    "Konfiguracja sprzętu została wgrana do CPU.",
  "event.diag.IpAddressConflict": "Wykryto konflikt adresów IP w sieci.",
  "event.diag.LinkDown": "Brak łącza Ethernet.",
  "event.diag.MemoryReset":
    "Reset pamięci: wszystkie dane programu zainicjalizowane, wszystkie wymuszenia usunięte.",
  "event.diag.ModbusException": "Wyjątek Modbus {exception}: {name}",
  "event.diag.ModeChange": "Stan pracy CPU zmienił się z {from} na {to}.",
  "event.diag.ModuleMismatch":
    "Moduł skonfigurowany różni się od zamontowanego (skonfigurowany: {configured}, wykryty: {installed}).",
  "event.diag.ModuleMissing":
    "Brak modułu: skonfigurowany moduł nie jest zamontowany lub nie odpowiada.",
  "event.diag.NoFieldPower":
    "Brak napięcia zasilania obciążenia (obiektowego) modułu.",
  "event.diag.NoRoute":
    "Brak trasy do zdalnego urządzenia: adres leży poza podsiecią PLC, a brama nie jest skonfigurowana.",
  "event.diag.Overflow":
    "Przepełnienie wejścia analogowego (sygnał powyżej zakresu pomiarowego).",
  "event.diag.PlcFault":
    "CPU w stanie FAULT: błąd wykonania zatrzymał program.",
  "event.diag.ProgramDownloaded": "Program został wgrany do CPU.",
  "event.diag.Startup.cold":
    "Restart zimny: wszystkie dane programu zainicjalizowane, łącznie ze zmiennymi retencyjnymi.",
  "event.diag.Startup.warm":
    "Restart ciepły: dane nieretencyjne zainicjalizowane, zachowane zmienne retencyjne: {retained}.",
  "event.diag.UnconfiguredModule":
    "Moduł jest zamontowany w pozycji, która nie jest skonfigurowana.",
  "event.diag.Underflow":
    "Niedomiar wejścia analogowego (sygnał poniżej zakresu pomiarowego).",
  "event.diag.WireBreak":
    "Przerwanie przewodu na wejściu analogowym (prąd pętli poniżej progu wykrywania).",
  "event.diagnostic": "{arrow} {diagnostic}",
  "event.fault": "{code}: {detail}",
  "event.firstScanCold": "Pierwszy cykl po starcie zimnym",
  "event.firstScanWarm": "Pierwszy cykl po starcie ciepłym",
  "event.force": "Wymuszenie {name} = {value}",
  "event.forceRemoved": "Usunięto wymuszenie z {name}",
  "event.hardwareDownloaded": "Wgrano konfigurację sprzętu.",
  "event.memoryReset":
    "Reset pamięci: dane programu zainicjalizowane, wymuszenia usunięte",
  "event.mode": "CPU {from} → {to} ({reason})",
  "event.net.arp": "✕ ARP: żadne urządzenie nie odpowiada dla {ip}",
  "event.net.conflict": "⚠ na adres {ip} odpowiada kilka urządzeń",
  "event.net.exception": "← wyjątek {code}h ({name}) TID {tid}",
  "event.net.noRoute": "✕ brak trasy do hosta (cel poza podsiecią, brak bramy)",
  "event.net.refused": "✕ połączenie odrzucone (TCP reset)",
  "event.net.request":
    "→ FC{function}: odczyt rejestrów ({quantity}) od adresu {address} (Unit ID {unit}), TID {tid}",
  "event.net.response": "← odpowiedź TID {tid} po {time} ms",
  "event.net.timeout": "✕ przekroczenie czasu TID {tid}",
  "event.op.door": "{device}: drzwi {state}",
  "event.op.doorClosed": "zamknięte",
  "event.op.doorOpened": "otwarte",
  "event.op.presence": "{device}: obiekt {state}",
  "event.op.present": "obecny",
  "event.op.press": "Naciśnięto {device}",
  "event.op.release": "Zwolniono {device}",
  "event.op.removed": "usunięty",
  "event.op.selector": "Przełączono {device} w położenie {position}",
  "event.process": "{label}: {event}",
  "event.process.dryRun": "praca na sucho: pompa pracuje przy pustym zbiorniku",
  "event.process.dryRunEnded": "koniec pracy na sucho",
  "event.process.highHigh": "poziom bardzo wysoki",
  "event.process.highHighCleared": "poziom bardzo wysoki ustąpił",
  "event.process.lowLow": "poziom bardzo niski",
  "event.process.lowLowCleared": "poziom bardzo niski ustąpił",
  "event.process.overflow":
    "przepełnienie: zbiornik jest pełny i nadal napełniany",
  "event.process.overflowEnded": "koniec przepełnienia",
  "event.process.overload": "zadziałał przekaźnik przeciążeniowy",
  "event.process.overloadReset": "przekaźnik przeciążeniowy zresetowany",
  "event.programDownloaded":
    "Program {name} został wgrany (wersja {revision}).",
  "event.reason.cold": "restart zimny",
  "event.reason.download": "wgranie",
  "event.reason.fault": "błąd wykonania",
  "event.reason.memoryReset": "reset pamięci",
  "event.reason.switch": "przełącznik trybu",
  "event.reason.warm": "restart ciepły",
  "event.reason.worldReset": "reset symulacji",
  "event.repair": "Naprawa: {label}",
  "event.sequence.aborted": "Sekwencja {sequence} przerwana w kroku {from}",
  "event.sequence.completed": "Sekwencja {sequence} zakończona",
  "event.sequence.deviation": "Sekwencja {sequence}: {deviation} (krok {step})",
  "event.sequence.faultCleared": "Sekwencja {sequence}: błąd skasowany",
  "event.sequence.held": "Sekwencja {sequence} wstrzymana w kroku {step}",
  "event.sequence.interlockActive":
    "Sekwencja {sequence}: blokada {interlock} aktywna w kroku {step}",
  "event.sequence.interlockCleared":
    "Sekwencja {sequence}: blokada {interlock} ustąpiła",
  "event.sequence.resumed": "Sekwencja {sequence} wznowiona w kroku {step}",
  "event.sequence.runStarted":
    "Sekwencja {sequence}: nowy przebieg w kroku {step} ({start})",
  "event.sequence.startBlocked":
    "Sekwencja {sequence}: start zablokowany ({reasons})",
  "event.sequence.started": "Sekwencja {sequence} uruchomiona",
  "event.sequence.timeout":
    "Sekwencja {sequence}: przekroczony czas kroku {step}",
  "event.sequence.transition":
    "Sekwencja {sequence}: przejście {transition}, krok {from} → {step}",
  "event.sequence.transitionBlocked":
    "Sekwencja {sequence}: przejście {transition} zablokowane ({reasons})",
  "event.warmStart":
    "Start ciepły: dane nieretencyjne zainicjalizowane, zachowane zmienne retencyjne: {retained}",
  "event.worldReset": "Symulacja przywrócona do początkowego stanu instalacji.",
  "hw.addressOverlap":
    "{a} i {b} używają tych samych bajtów {direction} {first}..{last}",
  "hw.binding.disabled":
    "{tag} jest przypisany do {channel}, który jest wyłączony",
  "hw.binding.noChannel":
    "{tag} jest przypisany do kanału {channel} modułu {module}, który ma tylko {count} kanałów",
  "hw.binding.noModule":
    "{tag} jest przypisany do pozycji {slot}, w której nie skonfigurowano modułu",
  "hw.binding.shared": "{tag} dzieli kanał z innym tagiem",
  "hw.binding.twice": "{tag} jest przypisany do więcej niż jednego kanału",
  "hw.channelMode":
    "{channel}: skonfigurowano {mode}, a schemat pokazuje {label} z sygnałem {signal}",
  "hw.detected.at": "{name} w pozycji {slot}",
  "hw.detected.bytes": "bajty {first}..{last}",
  "hw.detected.dark": "zamontowany, diody stanu nie świecą",
  "hw.detected.noField": "brak zasilania obiektowego (obciążenia)",
  "hw.expected.freeAddresses": "adresy nieużywane przez {module}",
  "hw.fieldPower":
    "{id} w pozycji {slot}: moduł odpowiada, ale brakuje jego zasilania obiektowego (obciążenia) 24 V",
  "hw.loopSupply":
    "{channel}: {label} wymaga: {wanted}, a kanał jest skonfigurowany jako: {configured}",
  "hw.missing":
    "{id} ({name}) jest skonfigurowany w pozycji {slot}, ale nie odpowiada tam żaden moduł",
  "hw.modulePower":
    "{id} w pozycji {slot}: moduł jest zamontowany, ale nie odpowiada; jego diody stanu nie świecą",
  "hw.slot": "kaseta {rack}, slot {slot}",
  "hw.unexpected":
    "{name} jest zamontowany w pozycji {slot}, której projekt nie konfiguruje",
  "hw.value": "{value}",
  "hw.wrongSlot":
    "{id} ({name}) jest skonfigurowany w pozycji {configured}, która jest pusta; {name} jest zamontowany w pozycji {installed}, której projekt nie konfiguruje",
  "hw.wrongType":
    "{id} w pozycji {slot}: skonfigurowano {name}, wykryto {detected}",
  "io.input": "wejść",
  "io.output": "wyjść",
  "led.AMBER": "bursztynowa",
  "led.GREEN": "zielona",
  "led.GREEN_FLASHING": "zielona migająca",
  "led.OFF": "wyłączona",
  "led.RED": "czerwona",
  "led.RED_FLASHING": "czerwona migająca",
  "loop.passive": "wejście pasywne",
  "loop.passive4wire": "wejście pasywne (4-przewodowe)",
  "loop.supplied": "pętla zasilana z wejścia",
  "loop.supplied2wire": "pętla zasilana z wejścia (2-przewodowa)",
  "mode.digital": "cyfrowy",
  "mode.disabled": "wyłączony",
  "module.product": "{product} ({name})",
  "module.unknown": "moduł",
  "sequence.deviation.conditionNotMet": "warunek przejścia niespełniony",
  "sequence.deviation.interlockViolated": "blokada nieprzestrzegana",
  "sequence.deviation.permissiveViolated":
    "start bez spełnionych warunków zezwalających",
  "sequence.deviation.progressAfterFault": "postęp mimo błędu",
  "sequence.deviation.progressDuringHold": "postęp podczas wstrzymania",
  "sequence.deviation.undefinedTransition": "niezdefiniowana zmiana kroku",
  "sequence.deviation.unknownStep": "nieznana wartość kroku",
  "sequence.start.attached": "program w trybie RUN",
  "sequence.start.cold": "start zimny",
  "sequence.start.warm": "start ciepły",
  "signal.0-10V": "0-10 V",
  "signal.0-20mA": "0-20 mA",
  "signal.4-20mA": "4-20 mA",
};

/** Codes whose Polish text equals the English text by design, with the reason. */
export const sameInPolish: Readonly<Record<string, string>> = {
  "check.net.online": "established Polish engineering term",
  "check.plc.fault": "PLC mode name and the runtime's technical message",
  "check.plc.faultEmpty": "PLC mode name",
  "check.plc.runEmpty": "PLC mode name",
  "event.alarm.activated": "the Polish word is the same",
  "event.mode": "PLC mode names and the CPU abbreviation",
  "signal.0-10V": "signal range with unit symbols",
  "signal.0-20mA": "signal range with unit symbols",
  "signal.4-20mA": "signal range with unit symbols",
};
