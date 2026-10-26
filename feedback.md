# PLC Lab — Independent Technical Audit

Audytor: niezależny principal engineer / audytor techniczny
Data audytu: 2026-10-26
Przedmiot: repozytorium `C:/Users/mjgom/Documents/PLC_Student` (GitHub: `Michael-projeCtSS/PLC-Lab`)
Zakres: stan zatwierdzony w HEAD `ed5ea38d2293a93629002680cbfd36d51778ac4e` (zawiera implementację M4B `db5e357e49090d8adcb14c6f847a0e37afdf6887`).
O ile nie zaznaczono inaczej, numery linii odnoszą się do plików w HEAD `ed5ea38`.

---

## 1. Executive verdict

**READY WITH REQUIRED FIXES**

Rdzeń projektu jest wyraźnie ponadprzeciętnej jakości. Semantyka PLC (transakcyjny skan, checked INT, skończony binary32 REAL, TIME jako dokładne u64 µs, TON na czasie symulacji), kompilator ST (ograniczony, deterministyczny, bez paniki na złośliwym wejściu), kontroler wykonania ciągłego (jeden mutex, lease/epoch, brak catch-up) oraz kontrakt IPC (ścisła walidacja po obu stronach, u64 jako stringi) są poprawne. Nie znalazłem błędu poprawności w semantyce PLC ani wyścigu, który pozwalałby wykonać skan bez ważnej dzierżawy (lease). Testy (307 debug + 245 release Rust, 192 frontend, 27 architektonicznych) przechodzą lokalnie na HEAD, a hostowany CI dla `db5e357` jest zielony na Windows (z prawdziwym WebView2) i Linux.

Nie rekomenduję jednak rozpoczynania oceniania (M5) bezpośrednio na obecnym modelu obserwacji. Powody:

1. **Trace i snapshot nie zawierają obrazu wejść, który faktycznie wykonał skan** (B1). Każdy wiersz `ScanCompleted` łączy wejścia przygotowane na *następny* skan z wyjściami *bieżącego* skanu. Dla Tanka oznacza to wiersz „LevelHigh = TRUE, InletValve = TRUE” dla programu `InletValve := NOT LevelHigh` — potwierdzone sondą. Ocenianie zbudowane na tym modelu utrwali przesunięcie przyczynowości o jeden skan we wszystkich kryteriach i golden reportach.
2. **Trace ma limit liczby wpisów, ale nie bajtów** (H1). Każde zdarzenie przechowuje pełny snapshot wszystkich zmiennych. Legalny program 40 KB (3 426 zmiennych BOOL, w granicach limitu 64 KiB) daje stronę trace o rozmiarze **113 MB JSON**.
3. **`desktop-session` staje się centralną, niepodzielną klasą orkiestracji** (H2). Typy transportowe IPC (`IpcError`, `RuntimeRole`) pełnią w niej rolę typów domenowych, a przepływ sterowania zależy od porównań stringów kodów błędów. Niezatwierdzona praca M5 w drzewie roboczym dokłada do tego crate’a logikę konkretnego ćwiczenia.
4. Występuje dryf metadanych i dokumentacji: `CoreInfo.execution == "manual-single-scan"`, README twierdzi „Run/Pause/Stop remain disabled”. Higiena repozytorium też wymaga uwagi: `main` stoi na M1C, a w drzewie roboczym leży niezatwierdzona, nieprzejrzana implementacja M5 (M8, M9).

Wszystkie te poprawki mają ograniczony zakres. Proponuję krótki kamień milowy wzmacniający, opisany w sekcji 20, a dopiero po nim M5.

---

## 2. What I inspected

### Stan repozytorium

- Gałąź robocza: `codex/m5-demo-001-grading`. HEAD = `ed5ea38` (commit dokumentacyjny M4B), rodzic `db5e357` (zwalidowana implementacja M4B).
- **Drzewo robocze NIE jest czyste**, wbrew briefowi. `git status` pokazuje 34 zmodyfikowane pliki i około 40 nieśledzonych. Jest to częściowa, niezatwierdzona implementacja M5: `crates/grading-core/src/{definition,evaluation,evidence,bounded}.rs`, `crates/desktop-session/src/assessment*.rs`, `exercises/demo-001/exercise.json` (571 linii), `tests/fixtures/demo-001/golden-reports.json` (10 763 linie), `docs/adr/0017-deterministic-exercise-grading.md`, `ExercisePanel.tsx`, a także zmiany w CI i w `CoreInfo`.
- Audytowałem **stan zatwierdzony** (HEAD). Wyeksportowałem go przez `git archive HEAD` do katalogu scratch poza repozytorium i tam uruchamiałem testy. Niezatwierdzoną pracę M5 jedynie przejrzałem pod kątem relacji z moimi ustaleniami (sekcje 4, 19). Nie testowałem jej i nie oceniam jej jakości.
- `main` = `8046273` (baseline M1C). Między `main` a `ed5ea38` jest 20 commitów na nakładających się gałęziach `codex/m2a…m4b`.

### Przejrzany kod źródłowy (pełne czytanie)

- `crates/plc-types` (values, identity, source, symbols, diagnostics, function_blocks, compatibility)
- `crates/plc-ir` (model, validation)
- `crates/plc-runtime` (runtime, memory, executor, plan, operators, function_blocks, errors)
- `crates/simulation-core` (clock), `crates/process-simulation` (Tank), `crates/trace-core`
- `crates/desktop-session` (lib, execution, process, trace)
- `crates/app-contract` (lib, session, compilation; pozostałe DTO selektywnie)
- `languages/st` (lexer, literals, parser, compiler, diagnostics)
- `apps/desktop/src-tauri` (commands, execution_driver, lib, main, capabilities, tauri.conf.json, build.rs)
- `packages/ipc/src/runtime.ts` (walidatory), `generated.ts` (selektywnie)
- `packages/ui/src`: `useRuntimeSession`, `useSessionTrace`, `coalesced-refresh`, `trace-history`, `LabShell`, `ProcessPanel`, `TimelinePanel`, `RuntimeVariables`, `runtime-values`, `source-spans`, `tank-example`, `useStCompilation`
- Testy: nazwy i kluczowe ciała w `desktop-session` (execution/trace/process/session), `plc-runtime`, `languages/st/tests/alpha.rs`, `process-simulation/tests/tank.rs`, `tests/e2e/continuous-smoke.mjs`, `desktop-smoke.mjs` (harness), `tests/architecture/execution.test.mjs`
- CI: `.github/workflows/validation.yml`, `quality.yml`, `scripts/ci.mjs`, `scripts/ci-windows-prerequisites.ps1`, `package.json`, `rust-toolchain.toml`
- Dokumentacja: README, ROADMAP, ARCHITECTURE, FRONTEND_BACKEND_BOUNDARY, CONTINUOUS_EXECUTION, RUNTIME_SEMANTICS, DESKTOP_RUNTIME_SESSION, PROCESS_SIMULATION, TRACE_MODEL, TON_SEMANTICS (fragmenty), `M1C_WINDOWS_CODE_INTEGRITY.md`
- Diff M4A→M4B: `git diff 89383b2 db5e357` (60 plików, +2632/−107), w tym capabilities, `Cargo.toml`, trace-core, desktop-session, editor.

### Faktycznie wykonane polecenia

Wszystkie na kopii HEAD w scratch, z repo-lokalnym toolchainem `.tools` (Rust 1.99.0), `--locked --offline`.

| Polecenie | Wynik |
|---|---|
| `cargo metadata --locked --offline` | OK |
| `cargo test` dla 12 pakietów (plc-types, plc-ir, plc-runtime, simulation-core, st-frontend, app-contract, desktop-session, process-simulation, trace-core, grading-core, network-core, plc-conformance) | **39 suit, 307 passed, 0 failed** |
| `cargo fmt --all -- --check` | OK |
| `cargo clippy` (9 pakietów core, `--all-targets -D warnings`) | OK; 14× ostrzeżenie ts-rs „failed to parse serde attribute `deny_unknown_fields`” |
| `cargo test --release` (7 pakietów jak w CI) | **25 suit, 245 passed, 0 failed** |
| `cargo test -p plc-lab-desktop` (host Tauri) | **NIE wykonano** — Windows Application Control zablokował wygenerowany build script (`indexmap … build-script-build`, *os error 4551*). Zgodnie z instrukcją nie obchodziłem blokady. |
| `vitest run` | **15 plików, 192 passed** |
| `tsc --noEmit` | OK |
| `eslint . --max-warnings 0` | OK |
| `prettier --check .` | OK |
| `node --test tests/architecture/*.test.mjs` | **27/27 pass** (wymaga cargo na PATH) |
| `gh run view 38613113329` | `headSha=db5e357…`, oba joby `success` |
| `gh run list` | kolejne runy M4B (`5c279ea`, `10671ed`, `db5e357`, `ed5ea38`) zielone; dwa wcześniejsze runy M4A czerwone przed naprawą |

Dodatkowo napisałem **sondy weryfikacyjne wyłącznie w kopii scratch**. Nie są częścią repozytorium i zostały usunięte po uruchomieniu.

| Sonda | Wynik |
|---|---|
| Trace Tanka, wiersz `ScanCompleted #8` | `LevelHigh=TRUE`, `InletValve=TRUE` |
| Reset Runtime po 5 skanach | `t=0`, `scans=0`, zbiornik 500‰, w procesie zawór „applied open”, `InletValve` PLC = FALSE |
| Ponowny Attach | poziom zbiornika wraca po cichu do 0 |
| Łańcuch `OR` w ST | maksymalnie 30 składników; przy 31 błąd `PLC-ST-010 Expression tree limit exceeded` |
| Dyskretyzacja Tanka (opróżnianie 0,5 s od pełnego) | dt=100 ms → 600‰; dt=3 ms → 668‰; dt=1 ms → 1000‰ (pompa w ogóle nie opróżnia) |
| Skala (3 426 zmiennych, 40 053 B) | release ≈ 314 µs/skan, snapshot JSON 443 459 B, **strona trace (256 wpisów) 113 513 681 B** |

### Czego nie wykonałem

- Natywnego smoke testu WebView2 (`npm run test:desktop`): build hosta jest lokalnie blokowany. Polegam na zielonym CI 38613113329.
- Nie pobierałem artefaktów CI (zrzuty ekranu, `result.json`). Ocena UX opiera się na kodzie źródłowym i scenariuszach testów e2e, nie na zrzutach.
- Nie odtwarzałem lokalnie linuksowego lane’u CI.
- Niezatwierdzonej pracy M5 nie testowałem.

---

## 3. What is particularly strong

1. **Transakcyjny skan jest prosty i poprawny.**
   - `Runtime::scan_transaction` (`crates/plc-runtime/src/runtime.rs:173-202`) klonuje pamięć zapisywalną i stan instancji FB.
   - Kolejność jest właściwa: wykonanie → sprawdzenie przepełnienia zegara → sprawdzenie licznika skanów → dopiero na końcu publikacja (`memory.commit`, `instances`, `completed_scans`, `clock`).
   - Każdy błąd odrzuca wszystko, łącznie ze stanem TON. Testy sprawdzają rollback z niedomyślnymi wyjściami, granice budżetu i przepełnienie licznika (`runtime.rs:297-311`, `tests/semantic_runtime.rs`, `ton_transaction_tests.rs`).
2. **Dokładne typy wartości z niezmiennikami wymuszonymi w konstruktorach i w deserializacji.**
   - `PlcReal` odrzuca NaN/Inf i normalizuje −0.
   - `PlcTime` serializuje się jako kanoniczny dziesiętny string (`plc-types/src/values.rs:47-135`).
   - Test `deserialization_cannot_bypass_value_invariants` zamyka furtkę przez serde.
   - Po stronie TS `parseRuntimeValue` (`packages/ipc/src/runtime.ts:83-110`) i `decimal()` z BigInt zachowują precyzję u64.
3. **Granica walidacji IR wymuszona typem.** `ValidatedProgram<'a>` da się uzyskać tylko z `validate()` (`plc-ir/src/validation.rs:21-64`), a `lower()` przyjmuje wyłącznie ten typ (`plc-runtime/src/plan.rs:75`). Plan wykonania jest osobnym, prywatnym modelem (`ExecStatementKind`) i zachowuje tylko użyte kotwice źródłowe.
4. **Determinizm z higieną.** W całej domenie są tylko `BTreeMap`/`BTreeSet`. `grep` nie znajduje `HashMap`, `Instant`, `SystemTime`, `rand` w crate’ach ani `Date.now`/`setInterval`/`setTimeout` w kodzie produkcyjnym UI. Czas hosta pojawia się wyłącznie w `execution_driver.rs` (`tokio::time::sleep`).
5. **Kontroler wykonania ciągłego jest minimalny i linearyzowalny.**
   - Wszystkie mutacje przechodzą przez jeden `Mutex<SessionSlot>`.
   - `continuous_tick` (`desktop-session/src/execution.rs:170-188`) autoryzuje się przez `session_id`, `mode == Running` i `active_lease == Some(lease)` — pod tym samym mutexem.
   - `paced_loop` (`apps/desktop/src-tauri/src/execution_driver.rs:6-31`) czeka przed każdym cyklem, nie ma catch-upu i kończy na `Stale`.
   - Ta prostota jest lepsza niż sprytniejsze alternatywy.
6. **Model źródeł gotowy na języki graficzne.** `SourceRef { document_id, element_id }` z opcjonalnym `TextSpan` (`plc-types/src/source.rs:84-133`) nie wymaga współrzędnych tekstowych. Kolumny liczone są w skalarach Unicode, z jawną konwersją do UTF-16 dla Monaco (`packages/ui/src/source-spans.ts`), spójną z lekserem (CRLF, LF i samotny CR).
7. **Kompilator ST jest ograniczony zasobowo.** Ma limity bajtów, tokenów, zagnieżdżenia i diagnostyk z twardymi sufitami (`languages/st/src/diagnostics.rs:20-63`). Przy złych danych daje błąd, nie panikę. Jest odzyskiwanie po błędzie składni, a słowa nieobsługiwane są rozpoznawane jawnie i dostają czytelny komunikat.
8. **Postawa bezpieczeństwa desktopu.**
   - Capabilities Tauri ograniczone do `core:event:allow-listen`/`allow-unlisten`.
   - Brak pluginów fs, shell, http.
   - Restrykcyjny CSP: `script-src 'self'`, bez `unsafe-eval`.
   - `unsafe_code = "forbid"` na poziomie workspace.
   - `continuous_tick` nie jest komendą IPC, więc lease jest niepodrabialny z przeglądarki.
9. **CI zaprojektowane z dbałością o łańcuch dostaw.**
   - Akcje przypięte do SHA, `permissions: contents: read`, `persist-credentials: false`.
   - `npm ci` i `--locked`, toolchain Rust przypięty do 1.99.0.
   - Bootstrapper WebView2 weryfikowany podpisem Authenticode Microsoft.
   - Natywny test z prawdziwym WebView2 na hostowanym Windows.
10. **Autorytet procesu po stronie Rust.** Fizyka Tanka jest całkowitoliczbowa (‰) i deterministyczna. Wejścia należące do procesu są odrzucane po stronie Rust (`desktop-session/src/lib.rs:250-252`). React tylko rysuje geometrię z danych snapshotu (`ProcessPanel.tsx:4-9`).

---

## 4. Blocking findings

### B1 — Obserwacja cyklu (snapshot i trace) nie zawiera obrazu wejść, który wykonał skan; wiersz łączy Inputs(t+dt) z Outputs(t)

Severity:
BLOCKER

Evidence:
- `crates/desktop-session/src/trace.rs:79-83`, `Observation::capture`: dla roli Input zwracane jest `core.staged_inputs()`, dla pozostałych `core.committed()`.
- `crates/plc-runtime/src/memory.rs:66-72` i `runtime.rs:65-70`: runtime przechowuje `input_image` i wystawia go w `RuntimeSnapshot::input_image()`, ale **żaden** konsument poza runtime go nie używa. `grep input_image` w `desktop-session`, `app-contract` i `packages` nic nie znajduje.
- `crates/desktop-session/src/execution.rs:73-85`, `execute_one_cycle`: `record(ScanCompleted)` następuje *po* `process.advance(...)`, a ten wywołuje `restage` czujników na następny skan (`process.rs:144-153`).
- Sonda: Tank, wiersz `ScanCompleted #8` → `LevelHigh = TRUE`, `InletValve = TRUE`, przy programie `InletValve := NOT LevelHigh;`.
- Testy utrwalają ten kształt jako „poprawny”: `tests/e2e/continuous-smoke.mjs:159-171` dla skanu 8 oczekuje `highLevel=true` i `inletValveOpen=true`.
- Niezatwierdzona praca M5: `crates/desktop-session/src/assessment_trace.rs:57-79` buduje `CycleFacts` z tych samych `o.variables`. Fakty oceniania łączą więc `LevelHigh/Start/Stop/EmergencyStop` przygotowane na następny skan z `InletValve/DrainPump` bieżącego skanu.

Problem:
Snapshot i każdy wpis trace są spójnym zdjęciem stanu *po* cyklu. Brakuje w nich jednak tego, co program faktycznie odczytał. Relację „wejście → wyjście” można odtworzyć tylko pośrednio, z wejść przygotowanych w *poprzednim* wpisie. To przestaje działać przy eviction, na granicach stron, po `RuntimeReset` lub `ProcessAttached`, i każdy konsument (Timeline, ocenianie, przyszły monitoring LAD) musiałby tę logikę implementować sam. Dokumentacja (`DESKTOP_RUNTIME_SESSION.md`: „Inputs show the staged editable value for the next attempted scan”) świadomie przyjęła ten kształt w M2B, gdy wejścia edytował tylko użytkownik. Po M3A, kiedy proces nadpisuje czujniki po każdym skanie, decyzja ta stała się semantycznie myląca.

Why it matters:
- **Edukacyjnie** Historical Inspector i panel Variables pokazują stany, które wyglądają jak błąd programu.
- **Dla oceniania** kryteria typu „po wykryciu LevelHigh zawór musi się zamknąć w ≤ N skanach” zostaną zakodowane z niejawnym przesunięciem o jeden skan. Późniejsza zmiana wymusi przebudowę wszystkich ćwiczeń i golden reportów (w drzewie roboczym jest już ich 10 763 linii).
- **Dla LAD/FBD** monitoring online musi pokazywać wartości, które *widział* skan.

Recommended direction:
- Rozszerzyć obserwację cyklu o rekord przyczynowy:
  - `scan_inputs`: obraz wejść użyty przez skan (`RuntimeSnapshot::input_image`);
  - `committed`: wyjścia i zmienne lokalne;
  - `applied_actuators`: polecenia przekazane procesowi;
  - `process_after` i `staged_next`: czujniki dla następnego skanu.
- W DTO trace dodać pole obrazu wejść skanu. Rozważyć, czy panel Variables w trybie z procesem ma pokazywać „PLC widział / następny skan”.
- Dodać test-niezmiennik: dla każdego `ScanCompleted` z programem `Q := NOT I` zachodzi `Q == NOT scan_inputs[I]`.
- Ewidencja oceniania ma używać wyłącznie `scan_inputs`.

Required before:
M5 (jakiekolwiek ocenianie behawioralne).

---

## 5. High-priority findings

### H1 — Trace ma limit wpisów, ale nie rozmiaru; pełny snapshot na każde zdarzenie powoduje nieograniczony koszt pamięci i IPC

Severity:
HIGH

Evidence:
- `crates/desktop-session/src/trace.rs:32-57`: `Observation` zawiera `Vec<ObservedVariable>` z `name: String` dla każdej zmiennej przy *każdym* zdarzeniu. `record` (`:226-243`) wywołuje to przy każdym skanie, stagingu i zdarzeniu sterującym.
- `TRACE_CAPACITY = 4096` (`trace.rs:8`). `TRACE_PAGE_MAX = 256` wpisów, bez limitu bajtów.
- Frontend: `useSessionTrace.ts:39-54` (do 16 stron na łańcuch odświeżenia) oraz `trace-history.ts:3-25` (lustro całej historii, przebudowa `Map` i `sort` z BigInt przy każdej stronie).
- Sonda: program 40 053 B (3 426 BOOL; limit źródła to 65 536 B, tokenów 16 384, węzłów walidacji 10 000). Snapshot JSON ma 443 KB, a **jedna strona 256 wpisów 113,5 MB JSON**. Szacunkowo bufor 4096 wpisów w backendzie zajmuje rzędu ~1 GB RAM, a frontend próbuje sparsować do 16 takich stron.

Problem:
„Bounded trace” jest ograniczony liczbą zdarzeń, nie bajtami. Koszt rośnie jak O(zdarzenia × zmienne) w pamięci backendu, w IPC i w pamięci WebView. Program przyjęty przez kompilator może doprowadzić aplikację do zawieszenia lub braku pamięci. AGENTS.md §3.4 wymaga ochrony przed nadmiernym zużyciem pamięci przez program ucznia.

Why it matters:
- Jest to wektor DoS, choć „self-inflicted” w aplikacji jednoosobowej.
- Bardziej istotne: ocenianie, replay i monitoring LAD/FBD przy setkach zmiennych odziedziczą ten model kosztowy.
- Izolowane sesje oceniania (projekt M5) mnożą bufory.

Recommended direction:
- **Natychmiast (tanio):** twardy limit liczby deklaracji lub łącznego rozmiaru obserwacji dla profilu Alpha w konfiguracji kompilatora lub sesji, plus limit bajtów na stronę trace.
- **Strukturalnie:**
  - nazwy, typy i role trzymane raz w deskryptorze sesji, nie w każdym wpisie;
  - trace oparty na zmianach (wartości zmienione, kluczowane `SymbolId`) z okresowymi keyframe’ami;
  - strony ograniczone bajtami;
  - frontend pobiera projekcje (wybrane sygnały, okno czasu) zamiast lustra całej historii.
- Rejestrator ewidencji oceniania powinien być osobny od pierścieniowego bufora UI (patrz sekcja 19).

Required before:
M5 (co najmniej limity rozmiaru i decyzja o osobnym rejestratorze ewidencji). Pełny trace oparty na zmianach — przed LAD.

### H2 — `desktop-session` staje się „god orchestration crate”; DTO IPC służą jako typy domenowe, a sterowanie zależy od stringów kodów

Severity:
HIGH

Evidence:
- `crates/desktop-session/src/lib.rs:34-55`: `LoadedSession` agreguje runtime, konfigurację, deskryptory zmiennych, proces, tryb, epoch, lease, trace, nazwy instancji i błędy sterownika.
- Ten sam crate:
  - kompiluje, waliduje i obniża program oraz ustala domyślne parametry zadania (`lib.rs:114-173`);
  - wiąże scenę po nazwach (`process.rs:39-82`);
  - nagrywa trace i projektuje DTO (`trace.rs`);
  - realizuje kontroler lease (`execution.rs`).
- Błąd domenowy to wszędzie `app_contract::IpcError`. `with_session` (`lib.rs:221-234`) zmienia stan sesji na podstawie `error.code == "APP-RUNTIME-INTERNAL"` i `"APP-PROCESS-INTERNAL"`. `VariableDescriptor.role` używa DTO `RuntimeRole` (`lib.rs:50-55`).
- Kierunek zależności: `desktop-session → app-contract → st-frontend, plc-runtime` (`crates/app-contract/Cargo.toml`).
- Niezatwierdzona praca M5 dokłada do tego samego crate’a `assessment.rs` (314 linii; zaszyte `"DEMO-001"`, `include_str!` ćwiczenia), `assessment_trace.rs` (wyszukiwanie zmiennych po nazwach `"Start"`, `"EmergencyStop"`…) oraz `assessment_tests.rs`.

Problem:
Każda nowa funkcja (ocenianie, nowe sceny, sieć, sesje LAD) trafia do jednego crate’a. Silnik symulacji nie jest dostępny bez zależności od kontraktu transportowego. Polityka błędów oparta na stringach jest krucha: zmiana kodu po cichu wyłącza latching błędu.

Why it matters:
Ocenianie potrzebuje bezgłowego silnika, nie adaptera IPC. Sieć potrzebuje wielu uczestników symulacji. Obecny kształt sprzyja dopisywaniu logiki konkretnych scen i ćwiczeń w miejscu, które powinno być generyczne.

Recommended direction:
- Wydzielić bezgłowy crate (np. `simulation-session` lub `lab-engine`). Należą do niego: Runtime, scena (komponenty procesu), rejestrator obserwacji i `execute_one_cycle`. Ma własny typowany błąd (`enum SessionError { Domain(..), Internal, ProcessInternal, … }`) i nie zależy od `app-contract`.
- `desktop-session` staje się adapterem: wersjonowanie IPC, mapowanie DTO, tryb i lease, paginacja.
- Runner oceniania korzysta bezpośrednio z silnika. `grading-core` konsumuje typy ewidencji, nie obserwacje sesji.
- Usunąć porównania stringów kodów z logiki sterowania.

Required before:
M5 (typowany błąd i granica silnika używana przez assessment). Pełny podział — przed siecią (NETWORK).

### H3 — Czas symulacji należy do PLC, a proces ewoluuje tylko jako efekt uboczny skanu; Reset Runtime cofa czas przy zachowanym procesie

Severity:
HIGH

Evidence:
- `crates/plc-runtime/src/runtime.rs:105-114`: `Runtime` posiada `clock: SimulationClock` (linia 109). `reset()` (`:204-211`) zeruje zegar i licznik.
- `crates/desktop-session/src/execution.rs:73-85`: proces krokuje wyłącznie po udanym skanie PLC. `lib.rs:268-277`: Reset Runtime zachowuje zbiornik i ponownie przygotowuje czujniki.
- Sonda: po 5 skanach i Reset Runtime mamy `t=0`, `scans=0`, zbiornik 500‰, a w procesie zawór nadal „applied open”, choć `InletValve` w PLC = FALSE.
- `trace-core/src/lib.rs:169-183` (test `ordering_is_sequence_even_when_scan_and_time_reset_or_repeat`) akceptuje regres czasu w historii.
- AGENTS.md §3.3 wymaga trybów `paused`, `single scan`, `real-time`, `accelerated`. Trybu accelerated (ani slow-motion) nie ma.

Problem:
Nie istnieje zegar świata ani scheduler symulacji. Czas semantyczny procesu jest tożsamy z zegarem jednego zadania PLC. W tym modelu nie da się wyrazić:
- „PLC w STOP, a instalacja dalej pracuje”;
- wielu sterowników;
- zdarzeń sieciowych niezależnych od skanu;
- monotonicznej osi czasu po Reset Runtime.

Why it matters:
- Kryteria czasowe w ocenianiu wymagają monotonicznego czasu lub jawnych segmentów.
- Symulacja sieci (M9) wymaga schedulera zdarzeń.
- Ważne scenariusze edukacyjne, np. przejście PLC w STOP i reakcja procesu, są dziś niemożliwe.

Recommended direction:
- W `simulation-core` wprowadzić scheduler/zegar świata. Zadanie PLC staje się uczestnikiem z własnym okresem skanu, a komponenty procesu krokują na tickach świata.
- Reset Runtime resetuje stan zadania PLC, ale nie czas świata. Alternatywnie, na krótką metę: jawny `time_base_epoch` lub `segment_id` w `TracePoint`.
- Opisać to w ADR (zmiana fundamentalna, §15 AGENTS.md).

Required before:
NETWORK (scheduler). Przed M5 wystarczy znacznik segmentu w trace oraz reguła, że assessment używa świeżych sesji bez resetów.

### H4 — Brak abstrakcji sceny procesowej; Tank jest zaszyty w kontrakcie, sesji, trace i UI

Severity:
HIGH

Evidence:
- `desktop-session/src/lib.rs:40`: `process: Option<process::AttachedTank>`.
- `trace.rs:56`: `process: Option<TankSnapshot>`.
- `process.rs:39-82`: nazwy `LevelLow`, `LevelHigh`, `LevelPercent`, `InletValve`, `DrainPump` zaszyte w kodzie.
- `ProcessSceneKind::TankFilling` i `ProcessSnapshotDto::TankFilling` w `app-contract`.
- `packages/ui/src/tank-example.ts:9-32` duplikuje reguły interfejsu w TS. `ProcessPanel.tsx` zawiera `TankRenderer`.
- `process-simulation` udostępnia tylko `TankFillingProcess` i stałe.

Problem:
Druga scena (przenośnik, silnik, cylinder) wymaga zmian w co najmniej sześciu miejscach i duplikacji reguł. Nie ma interfejsu komponentu (komendy, stan, sprzężenia zwrotne, usterki — §8 AGENTS.md) ani danych wiązań sterowanych z definicji sceny lub ćwiczenia.

Why it matters:
Bez tej abstrakcji każda nowa scena dokłada logikę do `desktop-session` (zob. H2). Ćwiczenia sterowane danymi (§10) wymagają scen sterowanych danymi.

Recommended direction:
- Wprowadzić typowane porty komponentów (digital/analog, in/out), definicję `Scene` (komponenty plus tabela wiązań do symboli PLC, wersjonowana, ładowana z danych) i generyczną obserwację procesu (`component_id → typed state`).
- Dodać zdarzenia procesu (Overflow, DryRun, Fault).
- Backend wystawia raport zgodności interfejsu, a frontend przestaje duplikować reguły.
- Tank pozostaje pierwszym komponentem. Nie budować edytora scen.

Required before:
druga scena procesowa (czyli przed poszerzeniem ćwiczeń poza DEMO-001). W tabeli sekcji 17: BEFORE LAD.

### H5 — Ścieżka pracy ucznia jest infrastrukturalna, a semantyka Stop/Pause odbiega od mentalnego modelu PLC

Severity:
HIGH (produktowo)

Evidence:
- `ProcessPanel.tsx:203-205` instruuje: „Use Example → Compile → Load Program → Attach Tank Scene → Run or Single Scan”.
- `LabShell.tsx:301-310`: Load jest nieaktywny bez udanego Compile, mimo że Rust i tak kompiluje ponownie. Każdy Load odłącza scenę (testy `process.rs`: `c.process.is_none()`).
- Stop zachowuje wyjścia i zamraża instalację (`CONTINUOUS_EXECUTION.md`: „Stop preserves memory, staged inputs, TON, clock, count, process…”). Pause blokuje Single Scan (`lib.rs:263-266`, `require_stopped`).
- W UI widać etykiety wewnętrzne: `LabShell.tsx:138` „FOUNDATION”, `:190` „deterministic 1× execution”, `:391` „Click speed does not affect simulation time”, `:562` „M4B · Continuous PLC + Process + Trace”, a w panelu Diagnostics „CORE CONNECTED … IPC v7”.
- Wejścia TIME wpisuje się w mikrosekundach (`runtime-values.ts:29-36`). Timeline to lista zdarzeń (`TimelinePanel.tsx:72-96`), nie przebieg czasowy.

Problem:
Początkujący musi zrozumieć cztery techniczne kroki na każdą iterację edycji. UI nie wyjaśnia, po co są rozdzielone. Stop w PLC Lab nie jest STOP-em sterownika (wyjścia nie przechodzą w stan bezpieczny) i jest praktycznie tożsamy z Pause, poza odblokowaniem edycji.

Why it matters:
M5 wprowadza ćwiczenia dla początkujących. Ocenianie nałożone na niezrozumiały workflow obniży wartość dydaktyczną całego produktu.

Recommended direction:
Patrz sekcja 16. W skrócie:
- jedna akcja „Wgraj do PLC” (Compile+Load+auto-Attach sceny ćwiczenia) jako kompozycja istniejących komend, bez zmiany granic;
- jawne nazwanie Stop jako „Zatrzymaj symulację” albo zaplanowanie profilu „PLC STOP”;
- Single Step w Pause;
- usunięcie etykiet milestone’ów z UI.

Required before:
M5 (ścieżka ćwiczenia musi być prowadzona).

### H6 — IR i runtime nie są jeszcze gotowe na obniżanie i monitoring LAD/FBD

Severity:
HIGH

Evidence:
- `crates/plc-ir/src/model.rs:19-32`: brak wyrażenia „odczyt wyjścia instancji FB” i brak symboli tymczasowych lub syntetycznych. Wyjścia TON są dostępne tylko przez wiązanie `Q => zmienna` (`CallBinding::Output`). Lekser nie zna `.`, więc `Timer.Q` daje błąd znaku (`lexer.rs:290-292`).
- `languages/st/src/parser.rs:414-440`: głębokość drzewa binarnego jest limitowana. Sonda: maksymalnie 30 składników `OR` w jednym wyrażeniu.
- `plc-ir/src/validation.rs:18-19`: `MAX_VALIDATION_DEPTH = 64`, `MAX_VALIDATION_NODES = 10_000`. Egzekutor jest rekurencyjny (`executor.rs:71-110`).
- Runtime publikuje tylko zatwierdzone zmienne. Nie ma wartości obliczonych per `SourceRef`, a AGENTS.md §7 wymaga „executed source locations”.
- `languages/st/src/compiler.rs:58-59` ustala `DocumentId(1)` i `PouId(1)`. `SourceElementId` pochodzą z licznika przejścia (`compiler.rs:124-137`), więc nie są stabilne między edycjami.
- `StatementKind` nie ma skoków (STL `JMP`/`JMPC`).

Problem:
Bez tych elementów:
- LAD power-flow (które styki przewodzą) dałoby się pokazać tylko przez ponowne obliczenie logiki w React, a to jest zakazane;
- rozgałęzienia wyjścia FB w FBD wymagają tymczasowych symboli, które pojawiłyby się jako zwykłe zmienne w Variables i trace;
- szerokie rungi z wieloma gałęziami równoległymi trafią w limity głębokości;
- STL ze skokami nie zmieści się w strukturalnym IR.

Why it matters:
To są dokładnie te miejsca, w których „jeden IR dla wszystkich języków” może wymusić przeprojektowanie w środku M6.

Recommended direction:
Przed M6 — ADR i implementacja:
1. `ExpressionKind::FunctionBlockOutput { instance, parameter }` albo jawne temporaries z `SymbolOrigin::Synthetic` (ukrywane w Variables i trace).
2. N-arne `And`/`Or` (lub zrównoważone obniżanie) plus iteracyjny egzekutor albo wyższe, uzasadnione limity.
3. Kanał „probe” w runtime: ograniczone wartości obliczone dla wybranych `SourceRef` w danym skanie, zapisywane jako dane monitoringu.
4. Model projektu i dokumentu z trwałymi identyfikatorami elementów i asemblerem multi-POU (alokacja `DocumentId`).
5. Decyzja ADR co do skoków STL: profil edukacyjny tylko strukturalny albo IR z CFG.

Required before:
LAD (M6).

---

## 6. Medium-priority findings

### M1 — Model Tanka zależy od kroku dyskretyzacji i po cichu obcina przepełnienie

Severity: MEDIUM

Evidence:
- `crates/process-simulation/src/lib.rs:53-61`: `product / MICROS_PER_SECOND` bez akumulacji reszty. `:94` zawiera `clamp(0, MAX_LEVEL)` bez zdarzenia.
- Test `fractional_contributions_truncate_independently_without_hidden_remainder` utrwala obcinanie jako zamierzone.
- Sonda (opróżnianie 0,5 s od pełnego): dt=100 ms → 600‰, dt=3 ms → 668‰, dt=1 ms → 1000‰ (pompa w ogóle nie działa).
- `FILL_RATE_PERMILLE_PER_SECOND = 1000`: pełny zbiornik w 1 s, czyli 10 skanów.
- Niezatwierdzone `exercise.json` zawiera pole `cycle_period_micros`.

Problem:
Fizyka nie jest niezmiennicza względem kroku. Przepełnienie, ważny błąd dydaktyczny i naturalne kryterium oceny, jest niewidoczne poza stanem „1000‰”. Napełnianie w 1 s jest za szybkie, żeby obserwować je przy 10 Hz.

Recommended direction:
- Stan w jednostkach sub-promilowych (np. µ‰ jako `u32`/`u64`) z akumulacją reszty.
- Liczniki i zdarzenia Overflow/DryRun.
- Parametry sceny (pojemność, przepływy) w danych sceny i wolniejsze tempo dydaktyczne.
- Do czasu naprawy: zablokować zmienny okres cyklu w ćwiczeniach.

Required before: M5, jeśli ćwiczenie definiuje okres cyklu lub kryterium przepełnienia.

### M2 — Niespójności Reset/Attach

Severity: MEDIUM

Evidence:
- `process.rs:199-204`: ponowny Attach zastępuje istniejący proces. Sonda: poziom wraca po cichu do 0.
- Po Reset Runtime proces pokazuje `inletValveOpen = true`, a wyjście PLC jest FALSE (sonda, `lib.rs:268-277`).

Recommended direction:
- Ponowny Attach odrzucać albo nazwać „Replace scene” z potwierdzeniem.
- Po Reset Runtime jawnie oznaczać aktuatory jako „nie zastosowano od resetu” albo zdefiniować i udokumentować inną semantykę.

Required before: M5 (scenariusze oceniania opierają się na semantyce resetu).

### M3 — Ścieżka błędów wykonania ciągłego jest niesymetryczna względem ręcznej

Severity: MEDIUM

Evidence:
- `execution.rs:170-188`: `continuous_tick` omija `with_session`, więc `APP-RUNTIME-INTERNAL` z `record()` lub obserwacji nie ustawia `driver_error`.
- `execution_driver.rs:24-27`: błąd ticka trafia wyłącznie do `eprintln!`. W buildzie release `main.rs:1` ustawia `windows_subsystem = "windows"`, więc nie ma konsoli i komunikat znika.
- `ScanError::Internal` (`execution.rs:90-93`) nie tworzy zdarzenia trace, a naruszenie niezmiennika wewnętrznego da się „naprawić” przez Reset Runtime jak zwykły błąd ucznia.
- Dodatkowo: przy błędzie kroku procesu skan PLC jest już zatwierdzony (`execution.rs:76-83`), a cykl nie zostaje zapisany w trace. W praktyce jest to nieosiągalne, ale cykl nie jest atomowy.

Recommended direction:
- Wspólna polityka błędów dla obu ścieżek.
- Zdarzenie `ApplicationFailure` w trace.
- Ostatni błąd sterownika widoczny w snapshot DTO.
- Log do pliku.
- Przygotowanie kroku procesu przed commitem PLC albo udokumentowanie częściowego commitu.

Required before: M5.

### M4 — Porządek intencji Load jest ustalany w kolejności dotarcia do backendu, nie w kolejności klienta

Severity: MEDIUM

Evidence:
- `lib.rs:89-112`: `reserve_load_intent` wykonuje się wewnątrz `spawn_blocking` (`commands.rs:68-79`).
- `useRuntimeSession.ts:153-172`: `invalidateSource` ustawia `busy.current = false` podczas trwającego żądania. Edycja w trakcie Load, po której następuje drugi Load, daje dwa równoległe `load_st_program`. Wątki mogą zarezerwować intencje w odwrotnej kolejności, a wtedy starszy kod źródłowy wygrywa w backendzie.
- UI zachowuje się bezpiecznie: pokazuje `STALE` lub błąd i wymaga ponownego Load. Mimo to gwarancja „starszy Load nie nadpisze nowszej intencji” obowiązuje tylko w porządku backendu.

Recommended direction:
Monotoniczny `source_revision` lub `request_id` od klienta w `LoadStProgramRequest`, odrzucanie niższych rewizji. Alternatywnie kolejka żądań w hooku.

Required before: LATER (niskie ryzyko), ale zmiana jest tania.

### M5 — Schemat IPC jest ręcznie duplikowany w walidatorach TS

Severity: MEDIUM

Evidence:
- `packages/ipc/src/runtime.ts:49-200`, `trace.ts`: ręczne walidatory odzwierciedlają serde.
- `generated.ts` zawiera tylko deklaracje typów.
- ts-rs ignoruje `deny_unknown_fields` (14 ostrzeżeń przy buildzie).
- Walidatory kodują konwencje przestrzeni kodów, np. `runtimeFault` ⇒ `PLC-RUNTIME-` (`runtime.ts:172-174`).

Recommended direction:
- Generowanie schematu (np. JSON Schema z Rust) i walidatorów.
- Testy kontraktowe przepuszczające próbki wyprodukowane przez Rust przez walidatory TS.
- Wyciszenie lub rozwiązanie ostrzeżeń ts-rs.

Required before: LAD (wzrost kontraktu).

### M6 — Lustro trace we frontendzie i renderowanie Timeline nie skalują się

Severity: MEDIUM

Evidence:
- `trace-history.ts:3-25`: `Map` i `sort` na każdej stronie.
- `TimelinePanel.tsx:72-96`: renderuje wszystkie do 4096 wierszy bez wirtualizacji, przy 10 Hz.
- Każdy wpis zawiera pełną listę zmiennych.

Recommended direction:
Wirtualizacja, projekcje po stronie backendu, widok przebiegów oparty na zdarzeniach zmian (razem z H1).

Required before: M5 (UX ćwiczeń) / LAD.

### M7 — Frontend duplikuje reguły zgodności interfejsu sceny

Severity: MEDIUM

Evidence:
`packages/ui/src/tank-example.ts:9-32` powiela `TankBindings::resolve` (`desktop-session/src/process.rs:39-82`), łącznie z treścią komunikatów.

Recommended direction:
Backend zwraca raport zgodności, np. w snapshot lub osobnej komendzie preflight (część H4).

Required before: druga scena.

### M8 — Higiena repozytorium i governance

Severity: MEDIUM

Evidence:
- `main` = `8046273` (M1C). Wszystkie zaakceptowane milestone’y leżą na nakładających się gałęziach (`git rev-list --count main..ed5ea38` = 20). CI uruchamia się na push do gałęzi, bez śladu PR.
- Drzewo robocze zawiera niezatwierdzoną implementację M5 (zmiany w CI, `CoreInfo`, kontrakcie IPC i UI), choć brief mówi, że jest czyste. ADR-0017 nosi status „Authorized by M5”.

Recommended direction:
- Zmergować zaakceptowane milestone’y do `main` przez PR z wymaganymi checkami.
- Włączyć ochronę `main`.
- Tagować zaakceptowane SHA.
- Pracę M5 przenieść na osobną gałąź (bez mergowania) do czasu decyzji technical leada.

Required before: NOW.

### M9 — Dryf dokumentacji i metadanych

Severity: MEDIUM

Evidence:
Szczegóły w sekcji 18. Najważniejsze:
- `CoreInfo.execution = ManualSingleScan` (`app-contract/src/lib.rs:39-44, 87`), zablokowany testem konformancji (`tests/conformance/tests/foundation.rs:13,16`);
- README (`:3`, `:7`, `:36`) twierdzi, że Run/Pause/Stop są wyłączone.

Recommended direction:
Poprawić teraz. Przebudować ARCHITECTURE/README z dziennika milestone’ów na opis stanu bieżącego, a historię przenieść do raportów.

Required before: NOW.

### M10 — Luki w CI i łańcuchu dostaw

Severity: MEDIUM

Evidence:
- Brak `cargo-deny`/`cargo audit` i `npm audit`/kontroli licencji.
- `node-version: 24` (tylko major).
- Natywny smoke używa `target/debug/plc-lab-desktop.exe` (`tests/e2e/desktop-smoke.mjs`), więc profil release nie jest testowany natywnie.
- `if-no-files-found: warn` dla artefaktów dowodowych.
- Brak podpisywania kodu. Smart App Control na komputerach uczniów zablokuje niepodpisaną aplikację tak samo, jak blokuje lokalne buildy właściciela.

Recommended direction:
- `cargo-deny` (advisories + licencje) i `npm audit --omit=dev` w CI.
- Pełna wersja Node.
- Smoke na buildzie release.
- `if-no-files-found: error` dla wymaganych dowodów.
- Przed dystrybucją: podpisywanie zaufanym certyfikatem (np. usługa typu Trusted Signing).

Required before: LATER (dystrybucja). Audyt zależności tanio teraz.

### M11 — Testy architektoniczne oparte na regexach po kodzie źródłowym

Severity: MEDIUM

Evidence:
`tests/architecture/execution.test.mjs:7-21`, np. `assert.match(domain, /active_lease == Some\(lease\)/)`, `/epoch\.checked_add\(1\)/`, liczenie wystąpień `runtime.single_scan(`.

Problem:
Sprawdzają obecność konkretnego zapisu, nie zachowanie. Neutralny refaktoring je psuje, a regresja semantyczna z zachowanym zapisem przejdzie.

Recommended direction:
Zostawić zakazy importów i zależności (te mają wartość, najlepiej przez `cargo metadata`). Asercje obecności wzorca zastąpić testami behawioralnymi.

Required before: LATER.

### M12 — Podzbiór ST Alpha jest za mały na program nauczania poza DEMO-001

Severity: MEDIUM (produktowo)

Evidence:
- `languages/st/src/lexer.rs:335-338`: `ELSIF`, `CASE`, `FOR` i inne są „unsupported”. Nie ma `XOR`, `MOD`, arytmetyki TIME ani odczytu `Timer.Q`.
- `plc-types/src/function_blocks.rs:18-20`: tylko `Ton` (brak `R_TRIG`/`F_TRIG`/`CTU`/`TOF`/`TP`).

Recommended direction:
Przed drugim ćwiczeniem: `ELSIF`, `CASE`, `Timer.Q`, `R_TRIG`/`F_TRIG`, `CTU`. Najlepiej po decyzjach IR z H6.

Required before: drugie ćwiczenie (po M5).

---

## 7. Low-priority / cleanup findings

- **L1** — REAL `/0` zgłaszany jest jako `PLC-RUNTIME-004` (non-finite), nie `PLC-RUNTIME-003` (division by zero) (`plc-runtime/src/operators.rs:80`). Komunikat dydaktycznie mylący.
- **L2** — Operatory relacyjne i równości mają ten sam priorytet (`parser.rs:449-454`), podczas gdy IEC 61131-3 nadaje relacyjnym wyższy. Dziś typy zwykle to odrzucają; warto udokumentować lub poprawić przed rozszerzeniem typów.
- **L3** — Maszyneria saturacji sekwencji u64 w trace (`trace-core/src/lib.rs:62-69, 79-93`) obsługuje stan praktycznie nieosiągalny. Można zostawić, ale nie rozwijać.
- **L4** — `app-contract` zawiera serwis kompilacji (`compilation.rs:96+`) i zależy od `st-frontend` oraz `plc-runtime` (tylko po `VERSION`). Istnieją dwie ścieżki kompilacji: `app_contract::compile_st` i `desktop_session::prepare_load`.
- **L5** — Kody `PLC-PROCESS-00x` płyną jako `IpcError` (`process.rs:51-71`), a UI rozgałęzia się po prefiksie (`LabShell.tsx:482`). Diagnostyka domenowa idzie kanałem błędów transportu.
- **L6** — Zatruty mutex jest nieodwracalny bez restartu (`lib.rs:81-88`). Fail-closed jest poprawny, ale UI powinno wprost poprosić o restart aplikacji.
- **L7** — Domyślny budżet 10 000 operacji jest równy `MAX_VALIDATION_NODES` 10 000. Bez pętli budżet działa dziś jako limit rozmiaru programu; trzeba to udokumentować przed dodaniem pętli.
- **L8** — Opis capability w `capabilities/main.json:4` („Foundation window: read-only application information”) jest nieaktualny.
- **L9** — Literały TIME nie obsługują ułamków (`T#1.5s`) ani podkreśleń (`T#1h_30m`). To dopuszczalne w Alpha, ale warto to pokazać w pomocy edytora.
- **L10** — Edge case: samotny surogat UTF-16 w edytorze Monaco prawdopodobnie da błąd deserializacji IPC zamiast diagnostyki kompilatora. Nie weryfikowałem.

---

## 8. Determinism audit

| Obszar | Werdykt | Uzasadnienie |
|---|---|---|
| **PLC Runtime** | **DETERMINISTYCZNY** | Tylko `BTreeMap`. Checked INT. Podstawowe operacje f32 według IEEE-754 (brak kontrakcji FMA w Rust, brak funkcji transcendentnych), kanoniczne zero. Zegar to tylko `checked_add` okresu. Brak czasu ściennego. Testy porównują wzorce bitowe REAL między Windows i Linux, debug i release. |
| **TON** | **DETERMINISTYCZNY** | Czas próbkowany na początku skanu (`executor.rs`, `now = clock.now()`). Stan aktywacji klonowany i commitowany transakcyjnie (`function_blocks.rs:37-65`). Semantyka: ET kwantowane do okresu cyklu, Q rośnie w pierwszym skanie, którego początek ≥ aktywacja + PT. Spójne i udokumentowane. |
| **Process (Tank)** | **DETERMINISTYCZNY, ale NIE niezmienniczy względem dt** | Arytmetyka całkowitoliczbowa i checked. Wynik zależy jednak od okresu cyklu przez obcinanie bez reszty (M1). |
| **Continuous execution** | **DETERMINISTYCZNY semantycznie** | Ciąg cykli identyczny z ręcznym (`execution.rs:248-314` porównuje prywatne snapshoty runtime 1:1, plus trace po projekcji pól sterujących). Czas hosta nie wpływa na dt, TON, Tank ani znaczniki trace. Uwaga: *który* skan zobaczy kliknięcie użytkownika zależy od czasu ściennego. To nieuniknione; trace to rejestruje, a ocenianie musi używać skryptowanych bodźców. |
| **Trace** | **DETERMINISTYCZNY dla danej sekwencji operacji** | Zawartość odtwarzalna, ale (a) czas cofa się po Reset Runtime (H3), (b) brak obrazu wejść skanu (B1), (c) brak nagłówka konfiguracji/seed, (d) eviction. Nie jest jeszcze logiem nadającym się do replay. |
| **Frontend** | **NIEAUTORYTATYWNY, poprawny** | Brak timerów i obliczeń PLC, tylko formatowanie. Kolejność historii po BigInt `sequence`. Tokeny generacji odrzucają stare odpowiedzi. Duplikacja reguł sceny (M7) to prezentacja, nie semantyka. |

---

## 9. Concurrency audit

Model: wszystkie mutacje sesji wykonują się pod jednym `Mutex<SessionSlot>`, więc każda operacja jest linearyzowalna w punkcie przejęcia mutexu. Analizowałem przeploty w tych punktach.

**Load.** `reserve_load_intent` (pod lockiem, wymaga STOPPED) → `prepare_load` (bez locka: kompilacja, walidacja, lowering) → `commit_prepared_load` (pod lockiem: sprawdza najnowszą intencję i ponownie STOPPED).
- Run między rezerwacją a commitem daje `APP-EXECUTION-ACTIVE`; test `stopped_guards_and_load_commit_race…` to pokrywa.
- Nowsza nieudana kompilacja unieważnia starszą przygotowaną (test).
- Ryzyko resztkowe: porządek intencji jest porządkiem backendu (M4).

**Run.** `start_execution` pod lockiem: wymaga Ready, inkrementuje epoch (`checked_add`), ustawia `Running` i `active_lease`, zapisuje `ExecutionStarted`. Sterownik jest spawnowany po zwolnieniu locka.
- Duplikat Run zwraca snapshot i `driver: None` (test).
- Pause przed startem workera: pierwszy tick (po 100 ms) dostaje `Stale`.
- Brak synchronicznego skanu w Run.

**Pause.** Pod lockiem: `mode = Paused`, `active_lease = None`, `ExecutionPaused`. Tick czekający na mutex dostaje potem `Stale`. Tick, który już trzymał mutex, kończy jeden cykl przed Pause (udokumentowane). Brak wycieku workera: worker kończy się na najbliższym ticku.

**Resume.** Nowy epoch i nowy lease. Stary worker, który obudzi się po Resume, widzi `active_lease != Some(old)` i dostaje `Stale`. Szybkie Pause/Resume tworzy krótko żyjące, ograniczone workery. Nigdy dwa nie wykonują cykli w tym samym epoch.

**Stop.** `halt()` czyści lease. Stan zachowany (test `control_trace_pause_stop_resume…`). Stop wymaga `Ready`, ale ścieżka „Faulted + Running” jest nieosiągalna, bo fault zawsze wywołuje `halt()`.

**Wake notifications.** `emit` następuje po ticku, poza lockiem. Kolejność wake względem odpowiedzi komend jest dowolna. Frontend traktuje wake wyłącznie jako wskazówkę:
- filtruje po `sessionId`, `runId` i lokalnym trybie `RUNNING` (`useRuntimeSession.ts:129-145`);
- przy trwającej mutacji ustawia `wakeQueued`;
- wynik odczytu zderzony z nowszą generacją jest odrzucany (`:103-113`);
- po potwierdzeniu Run wykonuje jeden dodatkowy odczyt, który łapie „wczesny terminalny tick” (naprawione w `10671ed`).

Nie znalazłem przeplotu prowadzącego do trwałej niespójności UI. Nieudany `emit` wywołuje `driver_failed` tylko dla aktywnego lease (test).

**Input staging.** `set_input` jest dozwolone w Running i Paused, trafia do `staged_inputs` i działa od następnego skanu (test `user_staging_during_run_and_pause…`). Wejścia należące do procesu są odrzucane w każdym trybie. Restage procesu nadpisuje wyłącznie własne czujniki.

**Session replacement.** Nowa generacja daje nowy `RuntimeSessionId`. Stary lease ma inne `session_id`, więc dostaje `Stale`. Odpowiedzi do starej sesji dostają `APP-RUNTIME-STALE-SESSION`. Wielowątkowy test `concurrent_replacement_cannot_apply_old_input_to_new_session` (Barrier) to potwierdza.

**Unload.** Wymaga STOPPED, a w stanach fail-closed UI dopuszcza Unload (`failedClosed`). Lease nieważny po Unload (test).

**Wnioski.** Kontroler jest poprawny. Znalezione słabości są drugorzędne:
- M3: asymetria obsługi błędów wewnętrznych w ścieżce ciągłej;
- M4: porządek Load;
- brak testu z *prawdziwymi* równoległymi wątkami `continuous_tick` i Pause/Stop/Load. Testy kontrolera są sekwencyjnymi przeplotami, co przy jednym mutexie jest uzasadnione, ale nie wykrywa np. regresji polegającej na zwolnieniu locka w środku cyklu.

---

## 10. Architecture / dependency audit

Faktyczny graf zależności (z `Cargo.toml`) jest acykliczny:

```
plc-types                          (leaf; serde)
├─ plc-ir           → plc-types
├─ simulation-core  → plc-types    (tylko SimulationClock)
├─ plc-runtime      → plc-types, plc-ir, simulation-core
├─ st-frontend      → plc-types, plc-ir
├─ process-simulation → plc-types
├─ trace-core       → plc-types
├─ grading-core     → plc-types, trace-core        (pusty: tylko VERSION)
├─ network-core     → plc-types, simulation-core, trace-core (pusty)
├─ app-contract     → plc-types, plc-runtime, st-frontend, ts-rs
├─ desktop-session  → app-contract, plc-types, plc-runtime, st-frontend,
│                     process-simulation, trace-core
└─ plc-lab-desktop  → app-contract, desktop-session, tauri, tokio
```

Ocena:

- **Liście są czyste.** `plc-runtime` nie zna procesu ani trace. `process-simulation` i `trace-core` zależą tylko od `plc-types`. To dobra baza.
- **`app-contract` siedzi w złym miejscu.** Crate z DTO powinien zależeć tylko od typów wartości, a zależy od kompilatora i runtime. `desktop-session` z kolei używa DTO jako swoich typów domenowych. Granica transport–domena jest więc odwrócona (H2).
- **`desktop-session` jest dziś zdrowy co do rozmiaru** (~1,2 tys. linii kodu bez testów), ale **niezdrowy co do odpowiedzialności**. Ma ich pięć: cykl życia sesji, kompilacja i obniżanie, wiązania sceny, nagrywanie i projekcja trace, kontroler wykonania. Projekt M5 dokłada szóstą i siódmą (ćwiczenia, ewidencja oceniania). To kierunek „god crate”. **Restrukturyzacja: minimalna przed M5** (silnik bezgłowy z typowanym błędem, assessment poza adapterem IPC), **pełna przed NETWORK**.
- **`grading-core` i `network-core` w HEAD są pustymi placeholderami** z zadeklarowanymi zależnościami. To przedwczesna struktura, ale tania. `simulation-core` zawiera tylko zegar. Nazwa obiecuje scheduler, którego nie ma (H3).
- **Granica React/Rust jest dobrze egzekwowana**, w sensie zakazów w testach architektonicznych i braku obliczeń w UI.

---

## 11. Structured Text / IR / future languages

**ST frontend jest solidną bazą dla Alpha.**
- Ręcznie pisany lekser z liczeniem skalarów.
- Parser z limitami i odzyskiwaniem po błędach.
- Kompilator z deterministycznym przydziałem ID (`SymbolId` i `SourceElementId` w kolejności przejścia).
- Identyfikatory nieczułe na wielkość liter, z zachowaniem oryginalnej pisowni.
- Poprawne `-32768`: składanie minusa z literałem (`compiler.rs:338-342`, `signed_literal`).
- REAL przez `parse::<f32>()` z odrzuceniem nieskończoności.
- TIME z malejącymi, niepowtarzalnymi jednostkami i checked mnożeniem.
- Obowiązkowa walidacja IR po kompilacji (`compiler.rs:96-101`) oraz ponownie przed loweringiem.
- Fuzz `bounded_malformed_inputs_never_panic_or_hang` jest skromny (500 × 96 znaków), ale nie znalazłem ścieżki paniki. Wszystkie `expect` dotyczą niezmienników ograniczonych limitami.

Kompilator powtarza część sprawdzeń typów walidatora, ale przez wspólne funkcje `plc_types::compatibility`. To nie jest duplikacja semantyki, tylko lepsze komunikaty.

**IR jest językowo neutralne w sensie składni.** Nie ma w nim nawiasów ani priorytetów ST. `SourceLanguage` obejmuje LAD/FBD/STL. Kotwice źródeł działają bez tekstu. Rozróżnienie IR i ExecutionPlan jest silne: osobne typy, a TON obniżany do dedykowanej instrukcji.

**Gotowość na LAD/FBD/STL: częściowa.** Bez wcześniejszych decyzji (H6) M6 trafi na:
1. **Odczyt wyjść FB w wyrażeniach i fan-out** (FBD, LAD z TON w środku rungu). Potrzebne jest wyrażenie `FunctionBlockOutput` albo symbole syntetyczne ukryte przed użytkownikiem.
2. **Szerokie OR/AND.** Drzewo binarne z limitem głębokości (sonda: 30 składników w ST; walidacja do 64 poziomów) zablokuje rungi z wieloma gałęziami równoległymi.
3. **Monitoring online.** Pokazanie przewodzenia styków wymaga wartości obliczonych per `SourceRef` publikowanych przez runtime. W przeciwnym razie UI musiałby ponownie obliczać logikę, co jest wprost zakazane.
4. **Tożsamość źródła graficznego.** `SourceElementId` w ST to licznik przejścia, niestabilny między edycjami. To jest OK dla tekstu, ale LAD/FBD potrzebują trwałych ID z modelu dokumentu oraz asemblera projektu z wieloma `DocumentId` i POU (`compile_st` ustala `DocumentId(1)` i `PouId(1)`).
5. **Krawędzie i stany ukryte** (styki P/N w LAD). Potrzebne są instancje FB generowane przez kompilator, z przestrzenią nazw nienachodzącą na nazwy użytkownika.
6. **STL/IL.** Akumulator (RLO) można obniżyć do wyrażeń. Skoki (`JMP`, `JMPC`) nie mieszczą się w strukturalnym IR, więc potrzebna jest decyzja ADR: profil edukacyjny tylko strukturalny albo IR z CFG.

**Rekomendacja przed M6:** jeden kamień milowy „IR & source model readiness” obejmujący punkty 1–5, plus ADR do punktu 6. Strukturalne `IF` jest wystarczające jako bieżąca reprezentacja semantyczna dla ST i LAD (cewki set/reset to IF/przypisanie). Nie trzeba przechodzić na CFG, o ile STL nie wymaga skoków.

---

## 12. Trace audit

**Co jest dobre:**
- `TraceSequenceId` jest odrębny od `RuntimeSessionId`, licznika skanów i zegara.
- Sekwencja jest ściśle rosnąca i nie podlega reużyciu. Eviction od najstarszych z dokładnym `dropped_count`.
- Wykrywanie luki kursora: `cursor < oldest − 1` (`trace-core/src/lib.rs:116-121`). `FutureCursor` jest jawnym błędem.
- Saturacja nie wpływa na wykonanie.
- Trace jest wyłącznie obserwacją, nagrywaną pod tym samym mutexem co mutacja, więc snapshot jest spójny.
- Fault ma proweniencję źródłową. Zdarzenia sterujące (Started/Paused/Resumed/Stopped) niosą tryb i `run_id`.

**Co jest niewystarczające jako baza dla oceniania i replay:**
1. Brak obrazu wejść skanu (B1).
2. Pełny snapshot na zdarzenie bez budżetu bajtów (H1). Trace jest modelem próbkowania stanu, a nie zmian, wbrew §7 AGENTS.md („input changes, output changes, variable changes”).
3. Czas niemonotoniczny po Reset Runtime, bez znacznika segmentu (H3).
4. Obserwacja procesu typowana na Tank (H4), więc zdarzenia sieciowe nie mają gdzie się zmieścić bez zmiany schematu.
5. Brak „executed source locations” i wartości probe, potrzebnych do wizualizacji źródła i LAD.
6. Brak nagłówka sesji (konfiguracja zadania, wersja programu i sceny), potrzebnego do replay.
7. Pierścień o pojemności 4096 jest buforem UI. Ewidencja oceniania nie może zależeć od jego retencji.

**Werdykt: obecny Trace nie jest wystarczająco stabilny, żeby ocenianie bezpośrednio na nim polegało.** Przed M5 należy poprawić punkty 1 i 3 (tanie) oraz rozdzielić „trace UI” od „rejestratora ewidencji” (sekcja 19). Punkty 2, 4 i 5 można zaplanować przed LAD i NETWORK, ale schemat ewidencji powinien je przewidywać.

---

## 13. Testing audit

### Tests I trust strongly

- `crates/plc-runtime/tests/semantic_runtime.rs`, `ton.rs`, `src/ton_transaction_tests.rs`: rollback z niedomyślnymi wyjściami, granice budżetu, przepełnienia, wzorce bitowe REAL, kolejność ewaluacji, gorliwe AND/OR. Asercje na dokładnych wartościach.
- `languages/st/tests/alpha.rs`, testy „pipeline_*”: prawdziwe ST → IR → runtime, w tym dokładna oś czasu TON i span faultu na oryginalnym wyrażeniu.
- `crates/desktop-session/src/execution.rs:248-314`, `manual_and_authorized_cycles_have_identical_memory_ton_clock_process_and_trace`: porównuje *prywatne* `runtime.snapshot()` obu usług 1:1 po każdym cyklu, z prawdziwym ST i Tankiem. Normalizuje tylko tryb, `run_id` i numer sekwencji, i to jest uzasadnione.
- Testy lease i stale (`control_trace_pause_stop_resume…`, `stopped_guards_and_load_commit_race…`, `host_failure_stops_only_its_lease…`): przy jednym mutexie sekwencyjne przeploty w punktach linearyzacji są właściwą metodą.
- `crates/desktop-session/tests/session.rs:406-460`: prawdziwe wątki z `Barrier` (8 skanów równolegle, wymiana sesji).
- `trace-core` (eviction, luki, saturacja) i `app-contract` (ścisłe parsowanie, test dryfu `generated.ts`).

### Tests I trust moderately

- Natywny smoke WebView2 (`tests/e2e/continuous-smoke.mjs`): prawdziwy, ale quiescence sprawdzana oknem 550 ms (wykryje ciągłe ticki, ale nie rzadkie). Działa tylko na buildzie debug. Część zapytań idzie bezpośrednio przez `__TAURI_INTERNALS__.invoke`, z pominięciem UI (poprawne do obserwacji).
- Testy hooków React (`useRuntimeSession.test.ts`, `continuous-execution.test.ts`): sensowne scenariusze wyścigów, ale na mockowanym IPC i fixture’ach DTO.
- `execution_driver.rs` (`paced_loop`): wstrzyknięte `wait`/`tick`/`notify` dowodzą kolejności, ale nie zachowania tokio i spawn_blocking. Lokalnie niewykonane (blokada SAC).
- Fuzz parsera: mały alfabet, 500 prób, jedyna asercja to brak paniki.

### Tests that may provide false confidence

- **Testy Tanka „scan 8/9”** (e2e i `trace_session.rs::actual_tank_trace_has_exact_full_cycle_states_including_scan_eight_and_nine`): dowodzą zgodności z bieżącą definicją obserwacji, która jest przyczynowo myląca (B1). Przechodzą, bo utrwalają problem.
- **`tests/conformance/tests/foundation.rs:13,16`** utrwala nieaktualne `ManualSingleScan` (M9).
- **Regexowe testy architektoniczne** (`tests/architecture/execution.test.mjs`): obecność zapisu ≠ poprawność (M11).
- Testy wstrzykujące stany nieosiągalne przez prywatne pola (`epoch = u64::MAX`, `completed_scans = u64::MAX`, `near_exhaustion_for_test`) są poprawne, ale dotyczą stanów, które w praktyce nie wystąpią. Nie należy liczyć ich jako „pokrycia odporności”.
- Liczba testów (~560) sugeruje więcej, niż weryfikują testy skali i zasobów, których nie ma.

### Important missing tests

1. Niezmiennik przyczynowy w trace: `outputs = f(scan_inputs)` dla każdego `ScanCompleted` (B1).
2. Testy zasobów: maksymalna liczba deklaracji, rozmiar bajtowy snapshotu i strony trace, pamięć bufora (H1).
3. Prawdziwie równoległe wątki: `continuous_tick` w pętli kontra Pause/Stop/Load/Unload z losowanymi opóźnieniami (stress, deterministyczny seed).
4. Property-based (np. `proptest`) dla leksera/parsera (round-trip spanów) i runtime (różnicowo IR bezpośrednio kontra ST).
5. Reset Runtime przy dołączonym procesie: semantyka czasu i aktuatorów (H3, M2).
6. Niezmienniczość Tanka względem dt (M1).
7. Ścieżka błędu wewnętrznego w trybie ciągłym widoczna w UI (M3).
8. Natywny smoke na buildzie release (M10).
9. Wydajność frontendu przy pełnej historii 4096 wpisów (M6).

---

## 14. Security audit

**Postawa ogólna: dobra jak na offline’ową aplikację edukacyjną; najsłabszy punkt to wyczerpanie zasobów.**

| Obszar | Ocena |
|---|---|
| Wykonanie kodu ucznia | Tylko VM w Rust: brak `eval` (grep), brak generacji kodu natywnego, brak dostępu do FS/sieci/procesów z poziomu programu PLC. |
| `unsafe` | `forbid` w workspace; grep nic nie znajduje. |
| Tauri capabilities | Minimalne: `core:event:allow-listen`/`allow-unlisten`, bez pluginów. 15 komend aplikacyjnych dostępnych z okna `main`. Przy braku zdalnej treści i ścisłym CSP to akceptowalne. Komend nie ograniczono manifestem aplikacji (opcjonalne utwardzenie). **Nie znalazłem zbędnych uprawnień.** |
| CSP | `default-src 'self'; script-src 'self'`, brak `unsafe-eval`, `connect-src ipc:` i `http://ipc.localhost`, `frame-src 'none'`. `style-src 'unsafe-inline'` jest potrzebne dla Monaco. Bez zewnętrznych URL. |
| Limity kompilacji | Źródło 64 KiB (sufit 1 MiB), 16 384 tokeny, zagnieżdżenie 32, 32 diagnostyki, 10 000 węzłów walidacji, głębokość 64. |
| Budżet skanu | 10 000 operacji na skan. Bez pętli w Alpha nie ma możliwości nieskończonej pętli. Rekurencja egzekutora ograniczona walidacją. Błędny program zatrzymuje zadanie (Faulted) bez zamrażania aplikacji. |
| Pamięć i trace | **Słabość (H1).** Legalny program prowadzi do ~1 GB trace w backendzie, 113 MB na stronę IPC i wielogigabajtowego lustra w WebView. |
| Powierzchnia IPC | Ścisła deserializacja (`deny_unknown_fields`, kanoniczne liczby dziesiętne), walidacja wersji przed mutacją, nieprzewidywalny lease (niedostępny przez IPC). Dodatkowe pola na najwyższym poziomie argumentów komendy (poza `request`) są ignorowane przez Tauri. Bez znaczenia. |
| Panika | Panika pod mutexem zatruwa go, więc wymagany jest restart aplikacji (fail-closed). Nie znalazłem paniki osiągalnej z poziomu programu ucznia. |
| Środowisko | `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS` (używane przez smoke) to cecha runtime WebView2. Lokalny atakujący kontrolujący środowisko i tak ma pełny dostęp. Informacyjnie. |

Rekomendacja: limit rozmiaru obserwacji i trace (H1) traktować jako wymaganie bezpieczeństwa z §3.4, nie jako optymalizację.

---

## 15. CI / release-confidence audit

**CI jest wiarygodne dla tego, co testuje, ale nie daje jeszcze pewności wydania.**

Mocne strony:
- Akcje przypięte do SHA, `contents: read`, `persist-credentials: false`, `npm ci`, `--locked`, toolchain z `rust-toolchain.toml` instalowany jawnie.
- Bootstrapper WebView2 weryfikowany podpisem.
- Brak `continue-on-error` i brak skipów.
- Wynik PASS/FAIL wynika z kodów wyjścia etapów (`scripts/ci.mjs:150-172`), a nie z interpretacji logów.
- Lane Windows uruchamia `npm run check`, czyli fmt, typecheck, ESLint, **clippy całego workspace**, testy Rust całego workspace, frontend, architekturę i build, oraz natywny smoke.
- Lane Linux to niezależny core z clippy (podzbiór), testami debug i release oraz architekturą.

Luki:
1. Brak audytu zależności i licencji (`cargo-deny`, `npm audit`).
2. Natywny smoke tylko dla buildu debug; profil release (z `windows_subsystem = "windows"`) nie jest uruchamiany natywnie.
3. `if-no-files-found: warn`: brak plików dowodowych nie zawiedzie joba.
4. `main` nie otrzymuje zaakceptowanej pracy (M8). CI chroni gałęzie, nie wydanie.
5. Node przypięty tylko do major.
6. Brak podpisywania binarki. Na maszynach z Smart App Control (jak u właściciela) aplikacja zostanie zablokowana, a to dotyczy dystrybucji do uczniów.
7. Natywne asercje opierają się na oknach czasowych. Są odporne na fałszywe FAIL przy wolnym runnerze, ale mogą dać fałszywy PASS przy rzadkich, opóźnionych tickach.

**Workflow z Windows Application Control.** Lokalnie dzisiaj testy core w katalogu tymczasowym uruchomiły się, a build script dla crate’a hosta został zablokowany (*os error 4551*). Blokowanie jest reputacyjne i niedeterministyczne. Poleganie na hostowanym Windows CI jako niezależnej walidacji natywnej jest **akceptowalne** na tym etapie. Ulepszenia bez osłabiania bezpieczeństwa:
- (a) szybka pętla core w WSL2: binarki ELF w maszynie wirtualnej nie podlegają SAC hosta;
- (b) natywny smoke w osobnej maszynie wirtualnej (Hyper-V) lub Microsoft Dev Box bez SAC, czyli w innej instancji systemu, bez zmian na hoście;
- (c) `workflow_dispatch` dla dowolnego SHA i PR;
- (d) docelowo podpisywanie buildów zaufanym certyfikatem.

Nie wyłączać SAC na hoście: tej decyzji nie da się cofnąć bez reinstalacji.

---

## 16. UX / educational product audit

**Obecnie UI wygląda bardziej jak debugger inżynierski niż środowisko nauki PLC.** To jest zrozumiałe przy budowie od fundamentów, ale trzeba to zmienić, zanim pojawią się ćwiczenia.

Obserwacje (z kodu i scenariuszy e2e; zrzutów nie oglądałem):

- **Workflow.** Use Example → (potwierdzenie) → Compile → Load Program → Attach Tank Scene → Run, czyli pięć do sześciu kliknięć na pierwszy efekt. Każda edycja kodu wymaga ponownie Compile → Load → Attach, bo Load zrzuca scenę. Rozdzielenie Compile, Load i Attach jest poprawne architektonicznie, ale UI nie wyjaśnia, po co istnieje.
- **Pojęcia.** Komunikaty takie jak „Inputs are staged for the next scan”, „Actuators show the last applied interval”, „deterministic 1× execution”, „Budget: 10000”, „IPC v7” są precyzyjne dla inżyniera i nieprzejrzyste dla początkującego.
- **Run/Pause/Stop.** Pause i Stop różnią się dla użytkownika tylko odblokowaniem edycji. W Pause nie można zrobić Single Scan, choć „pauza i krok” to naturalny wzorzec debuggera. Stop nie odpowiada STOP-owi sterownika: wyjścia nie przechodzą w stan bezpieczny, a świat stoi.
- **Tank.** Wizualizacja jest czytelna (zawór, pompa, progi, poziom), ale cały proces trwa 1 s (10 skanów). Uczeń nie zdąży zobaczyć przyczyny i skutku. Przepełnienie nie jest sygnalizowane. Opóźnienie sprzężenia zwrotnego (zamknięcie zaworu skan później) to świetny materiał dydaktyczny, ale dziś objawia się jako „sprzeczny” wiersz w historii (B1).
- **Variables.** Są użyteczne. Wejścia BOOL to checkboxy (przełączniki), a ćwiczenia Start/Stop potrzebują przycisków chwilowych. TIME wpisuje się w µs.
- **Timeline.** To lista zdarzeń z inspektorem, a nie oś czasu. Przy Run rośnie o 10 wierszy na sekundę i szybko przytłacza. Dla nauki znacznie cenniejszy jest widok przebiegów (analizator stanów logicznych dla BOOL I/O plus trend poziomu).
- **Diagnostyka.** Diagnostyka kompilatora i runtime jest odkrywalna (klikalne, ze spanem w edytorze). Panel Diagnostics miesza jednak informacje o połączeniu z rdzeniem z błędami ucznia.
- **Layout.** Siedem stałych paneli, w tym puste Project i Network. Edytory LAD i FBD potrzebują dużej kanwy, a stała siatka się nie przeskaluje.

Konkretne sugestie (bez zmian architektury):

1. Główna akcja **„Wgraj do PLC”** (Compile + Load + auto-Attach sceny ćwiczenia) jako kompozycja istniejących komend. Tryb zaawansowany z osobnymi krokami ukryty pod przełącznikiem.
2. Przełącznik trybu w stylu PLC: **RUN/STOP** plus osobne przyciski debuggera **Pauza / Krok / Wznów** (Single Scan dozwolony w Pause).
3. **Monitoring online w edytorze:** wartości obok zmiennych ST i podświetlenie wykonanej gałęzi IF. Wymaga danych z runtime (H6, punkt 3).
4. **Timeline jako przebiegi** z kursorem czasu. Lista zdarzeń jako widok drugorzędny.
5. Tank: wolniejsze tempo (10–20 s napełniania) albo regulacja tempa symulacji (accelerated/slow-motion z §3.3), wskaźnik przepełnienia, strzałki przyczyny i skutku („PLC widział LevelHigh=FALSE → zawór OTWARTY”).
6. Urządzenia wejściowe jako **przyciski chwilowe i przełączniki**, a nie checkboxy. TIME wpisywane w notacji `T#…`.
7. Usunąć z UI etykiety milestone’ów i szczegóły IPC (przenieść do „O programie”).
8. Dockowany lub kartowy układ paneli przed M6.

---

## 17. Scalability / technical debt

| Issue | Current impact | Future impact | Fix timing |
|---|---|---|---|
| Pełny snapshot wszystkich zmiennych (z nazwami String) na każde zdarzenie trace | pomijalny dla przykładów | O(zdarzenia×zmienne); 113 MB na stronę przy 3,4 tys. zmiennych; ~1 GB bufora | BEFORE M5 |
| Strony trace ograniczone liczbą wpisów, nie bajtami | brak | zawieszenie IPC i WebView | BEFORE M5 |
| Lustro całej historii we frontendzie, `Map` + `sort` z BigInt na stronę | małe | O(n log n) na wake przy 10 Hz, GB w JS heap | BEFORE M5 |
| Timeline bez wirtualizacji (do 4096 wierszy) | widoczne po minutach Run | zacinanie UI | BEFORE M5 |
| Brak obrazu wejść skanu w obserwacji | mylące wiersze | błędna przyczynowość w ocenianiu i LAD | BEFORE M5 |
| Polityka błędów przez porównania stringów kodów | brak | ciche wyłączenie latchingu | BEFORE M5 |
| Tank: obcinanie bez reszty (zależność od dt), brak zdarzenia overflow | brak (stałe 100 ms) | fizyka zależna od cyklu ćwiczenia | BEFORE M5 |
| `runtime.snapshot()` (klon 4 map + FB) wywoływane 4–6× na cykl (preflight, actuators, restage, capture) | µs | O(zmienne)×k na skan | BEFORE LAD |
| `begin_scan` klonuje pamięć zapisywalną i instancje na każdy skan | µs | O(zmienne) na skan (akceptowalne długo) | LATER |
| Pełny snapshot DTO na każdy wake (443 KB przy 3,4 tys. zmiennych) | brak | ~4,4 MB/s IPC przy 10 Hz | BEFORE LAD |
| Drzewo binarne + rekurencja; limit 30 składników OR; 64 poziomy; 10 000 węzłów | ogranicza kod ucznia | blokuje szerokie rungi LAD i duże programy | BEFORE LAD |
| Brak wartości probe per `SourceRef` | brak monitoringu w edytorze | LAD power-flow niemożliwe bez obliczeń w UI | BEFORE LAD |
| Niestabilne `SourceElementId` (licznik przejścia), stałe `DocumentId(1)`/`PouId(1)` | brak | brak trwałej tożsamości elementów graficznych, brak multi-POU | BEFORE LAD |
| Tank zaszyty w sesji, trace, DTO i UI (brak abstrakcji sceny) | brak | każda scena to zmiany w 6+ miejscach | BEFORE LAD (przed drugą sceną) |
| Ręczne walidatory TS powielające schemat Rust | brak | dryf i koszt przy wzroście kontraktu | BEFORE LAD |
| Jeden globalny `Mutex` na usługę sesji | brak | kontencja przy wielu sesjach, assessment i sieci | BEFORE NETWORK |
| `SimulationClock` własnością PLC; brak schedulera świata; regres czasu po Reset | myląca historia | sieć i wiele węzłów niemożliwe | BEFORE NETWORK |
| `desktop-session` jako centralny crate z DTO jako typami domenowymi | brak | każda funkcja ląduje w jednym miejscu | BEFORE M5 (minimum), BEFORE NETWORK (pełny podział) |
| Liniowe filtrowanie strony w `TraceBuffer::page` | brak | pomijalne przy 4096 | LATER |
| `spawn_blocking` na tick + 2 zapytania IPC na wake | brak | pomijalne przy 10 Hz | LATER |

---

## 18. Documentation drift

Dokumenty **mylące teraz** (nie tylko historyczne):

1. **`CoreInfo.execution = "manual-single-scan"`**: `crates/app-contract/src/lib.rs:39-44, 87`, `packages/ipc/src/generated.ts:9`, walidator `packages/ipc/src/index.ts:39`. Utrwalone testami `tests/conformance/tests/foundation.rs:13,16`, `packages/ipc/src/index.test.ts:13`, `LabShell.test.tsx`, `RuntimeShell.test.tsx`. **Potwierdzone:** metadane nie mówią o wykonaniu ciągłym. Niezatwierdzona praca M5 to poprawia, w HEAD — nie.
2. **`README.md`**: `:3` opisuje M3A i „Run/Pause/Stop remain disabled”; `:5-7` opisuje M2B i „IPC v4… Run/Pause/Stop remain disabled”; `:36` mówi „M1B requires technical-lead code review before M1C. Desktop execution controls remain unavailable.” Wszystkie trzy są nieaktualne.
3. **`docs/architecture/FRONTEND_BACKEND_BOUNDARY.md:56`**: tabela CoreInfo („contractVersion: 2; execution: 'unavailable'”) i `:21` („contract advances to v4… manual-single-scan”). Aktualny kontrakt to v7.
4. **`docs/architecture/ARCHITECTURE.md`**: dziennik milestone’ów (M4B na górze, M0 na dole), w którym kolejne sekcje deklarują „Execution controls and process/network/timeline/grading remain unavailable” (`:29`, `:114`). Czytelnik nie wie, co obowiązuje dziś.
5. **`docs/ROADMAP.md`**: tabela łączy „M4 Trace and Education” z ocenianiem, które przesunięto do M5. Sekcja „M1 progress” wciąż mówi „Trace schema / recording: Not implemented”, „M1C… review pending”.
6. **`apps/desktop/src-tauri/capabilities/main.json:4`**: „Foundation window: read-only application information”. Okno wykonuje mutujące komendy sesji.
7. **UI**: `LabShell.tsx:138` badge „FOUNDATION”, `:562` „M4B · Continuous PLC + Process + Trace”, panel Project „when project support is available”.
8. **`docs/spec/RUNTIME_SEMANTICS.md` i `PLC_RUNTIME_SEMANTICS.md`**: dwa pliki o mylnie podobnych nazwach (warstwa aplikacji vs rdzeń). Warto przemianować pierwszy na `APPLICATION_EXECUTION_SEMANTICS.md`.
9. **Brief audytu** zakłada czyste drzewo robocze, a w drzewie jest niezatwierdzona praca M5 (M8).

Ogólnie: dokumentacja jest bardzo szczegółowa, ale napisana gęstym „językiem raportu” i narasta przez dopisywanie. Rekomendacja: README i ARCHITECTURE jako opis *stanu bieżącego*, historia milestone’ów wyłącznie w `docs/M*_REPORT.md`.

---

## 19. M5 grading readiness

**Should M5 grading start now?**

**YES, BUT AFTER** krótkiego kamienia milowego wzmacniającego obserwację i ewidencję (sekcja 20): B1, minimum H1, typowany błąd i granica silnika z H2, znacznik segmentu czasu z H3, M1 (jeśli kryteria używają przepełnienia lub okresu cyklu), M3, M8 i M9.

Odpowiedzi na pytania:

1. **Czy architektura jest dojrzała na ocenianie?** Rdzeń wykonania tak. Model obserwacji jeszcze nie (B1, H1, H3).
2. **Trace bezpośrednio czy dedykowana projekcja ewidencji?** **Dedykowana, stabilna, wersjonowana projekcja semantyczna.** Ma zawierać per cykl: `scan_inputs`, `committed outputs`, zdarzenia procesu i stan po kroku, zdarzenia fault oraz czas semantyczny z segmentem. Rejestrator powinien działać obok trace UI, w izolowanej sesji, bez retencji pierścieniowej i bez wyszukiwania zmiennych po nazwach w kodzie (wiązania z definicji ćwiczenia, rozwiązane do `SymbolId` raz). `grading-core` zna tylko typy ewidencji.
3. **Czy trace zawiera właściwe informacje?** Nie w pełni: brak obrazu wejść skanu, brak zdarzeń przepełnienia i suchobiegu, brak segmentu czasu, nazwy duplikowane w każdym wpisie.
4. **Czy ocenianie stworzy niezdrowe sprzężenie z `desktop-session`?** Tak, jeśli pójdzie drogą widoczną w niezatwierdzonej pracy: `assessment.rs` i `assessment_trace.rs` wewnątrz `desktop-session`, zaszyte `DEMO-001`, wyszukiwanie po nazwach. Runner oceniania powinien używać bezgłowego silnika (H2).
5. **Czy wprowadzać teraz abstrakcję Exercise/Scenario?** Tak, minimalną i danowo-sterowaną: definicja ćwiczenia (wersja, interfejs, scena, kryteria) i scenariusz (stan początkowy, skryptowane bodźce na indeksach skanów, długość). Bez generycznego edytora i bez systemu autorskiego.
6. **Czy DEMO-001 Tank Filling to właściwe pierwsze ćwiczenie?** Jako dowód pionowego przekroju — tak (wymaga tego §19 AGENTS.md). Jako *pierwsza* lekcja dla początkującego jest trudniejsza niż „Start/Stop silnika z podtrzymaniem”. Rekomenduję DEMO-001 z wolniejszą fizyką i kryterium przepełnienia, a jako lekcję 0 prosty start/stop (ta sama infrastruktura, bez procesu albo z lampką).
7. **Czy persystencja projektu przed ocenianiem?** Nie dla DEMO-001 (ćwiczenie wbudowane). Minimalny zapis i odczyt kodu ucznia (ADR-0008) powinien jednak nastąpić przed drugim ćwiczeniem, bo utrata pracy po zamknięciu aplikacji jest dla ucznia dotkliwa.
8. **Czy najpierw dodatkowa semantyka PLC?** Dla DEMO-001 nie (BOOL + TON wystarczają). Przed kolejnymi ćwiczeniami tak: `ELSIF`, `CASE`, `Timer.Q`, `R_TRIG`/`F_TRIG`, `CTU` (M12), najlepiej po decyzjach IR (H6).
9. **Czy wykonanie ciągłe wymaga dalszego utwardzenia?** Tylko punktowo (M3, M4). Ocenianie i tak nie powinno używać trybu ciągłego (skryptowane cykle ręczne, jak zakłada ADR-0017, są słuszne).
10. **Czy brakuje ważniejszego fundamentu?** Tak, i to jest główny wniosek audytu: **semantyka obserwacji, czyli co dokładnie zdarzyło się w cyklu.** To od niej zależą ocenianie, Timeline, replay i monitoring LAD. Drugi fundament, scheduler świata (H3), jest potrzebny przed siecią, ale nie blokuje M5.

**Uwaga o niezatwierdzonej pracy M5.** Przejrzałem ją powierzchownie. Kierunek ADR-0017 jest w wielu punktach słuszny: izolowane sesje, brak czasu ściennego, `grading-core` bez zależności od DTO i sesji, ograniczone raporty. Dziedziczy jednak B1 (`CycleFacts` z wejść przygotowanych na następny skan) i pogłębia H2. Rekomenduję nie mergować jej w obecnej postaci, tylko oprzeć na projekcji ewidencji z M4C.

---

## 20. Recommended next milestone

**M4C — Observation & Evidence Hardening (przed ocenianiem)**

**Objective.**
Ustalić i przetestować jednoznaczną, przyczynową semantykę obserwacji cyklu oraz stabilną granicę ewidencji, zanim jakiekolwiek ocenianie lub golden reporty się od niej uzależnią.

**Why now.**
To ostatni moment, w którym zmiana schematu obserwacji jest tania. Po M5 będzie wymagała przebudowy ćwiczeń, kryteriów i golden reportów. Dodatkowo naprawia realny problem zasobowy (H1) i dryf metadanych.

**What it should contain.**
1. Obraz wejść skanu w obserwacji cyklu i w DTO trace. Niezmiennik przyczynowy w testach. Aktualizacja testów Tanka 8/9 do nowej, jawnej semantyki (B1).
2. Limity rozmiaru: liczba deklaracji lub rozmiar obserwacji dla profilu Alpha, limit bajtów na stronę trace, nazwy w deskryptorze sesji zamiast w każdym wpisie (H1, część).
3. Znacznik segmentu czasu (`time_base_epoch`) w `TracePoint`. Udokumentowana semantyka Reset Runtime względem procesu i aktuatorów. Ponowny Attach jawny (H3 część, M2).
4. Zdarzenia procesu Overflow i DryRun. Akumulacja reszty w Tanku albo jawne zablokowanie zmiennego okresu cyklu (M1).
5. Typowany błąd sesji zamiast porównań stringów. Wspólna polityka błędów manual i continuous, zdarzenie `ApplicationFailure` (H2 część, M3).
6. Wydzielenie bezgłowego API silnika (cykl, scena, rejestrator) bez zależności od `app-contract`. `desktop-session` jako adapter (H2, minimalny zakres).
7. Typy ewidencji (`CycleEvidence`, `ProcessEvent`, `FaultEvidence`) w małym, neutralnym module lub crate. Projekcja z rejestratora silnika, bez wyszukiwania po nazwach.
8. `CoreInfo.execution` → manual + continuous. Przepisanie README, ARCHITECTURE i FRONTEND_BACKEND_BOUNDARY do stanu bieżącego (M9).
9. Higiena repo: merge zaakceptowanych milestone’ów do `main` przez PR, ochrona `main`, przeniesienie niezatwierdzonego M5 na osobną gałąź (M8). Opcjonalnie `cargo-deny` w CI.

**What it must NOT contain.**
Reguł oceniania, definicji ćwiczeń i UI ćwiczeń. Nowych języków i nowych FB. Sieci. Persystencji. Nowych scen. Redesignu UI. Pełnego trace opartego na zmianach (wystarczą limity i deskryptory). Zmian semantyki ST, IR, runtime, TON i REAL.

---

## 21. Recommended next 3 milestones

1. **M4C — Observation & Evidence Hardening** (jak w sekcji 20).
2. **M5 — DEMO-001 Tank Filling + deterministyczne ocenianie behawioralne + prowadzony workflow ćwiczenia.**
   - Ocenianie na projekcji ewidencji z M4C.
   - Minimalna abstrakcja Exercise/Scenario sterowana danymi.
   - Runner na bezgłowym silniku.
   - Akcja „Wgraj do PLC” i panel ćwiczenia; przyciski chwilowe dla Start/Stop/E-Stop.
   - Wolniejsza fizyka DEMO-001.
   - Na tym etapie bez trybu accelerated w UI (assessment i tak działa bez tempa).
3. **M5B — IR & Source-Model Readiness (przed LAD).**
   - Odczyt wyjść FB / symbole syntetyczne.
   - N-arne AND/OR lub iteracyjny egzekutor.
   - Kanał probe wartości per `SourceRef` (monitoring w edytorze ST jako pierwszy konsument).
   - Model projektu i dokumentu z trwałymi ID elementów i asemblerem multi-POU.
   - Minimalna persystencja projektu (ADR-0008).
   - `ELSIF`, `CASE`, `Timer.Q`, `R_TRIG`/`F_TRIG`, `CTU`.
   - ADR o skokach STL.
   - Abstrakcja sceny procesowej (H4), jeśli planowane są nowe sceny.

Dopiero po nich **M6 LAD**. Scheduler świata (H3) i rozdzielenie `desktop-session` w pełnym zakresie należy zaplanować przed M9 (sieć).

---

## 22. Things I would NOT change

1. **Jeden runtime i jeden IR** dla wszystkich języków. `ValidatedProgram` jako jedyna droga do loweringu.
2. **Transakcyjny skan:** clone → execute → check clock/count → commit, z rollbackiem TON i brakiem auto-reset wyjść przy faulcie.
3. **Typy wartości:** checked INT16, skończony binary32 z kanonicznym zerem, TIME jako u64 µs, kanoniczne stringi u64 w IPC i ścisła walidacja po obu stronach.
4. **Kontroler wykonania:** jeden mutex, prywatny i niepodrabialny lease z epoch, wake tylko jako identyfikator, wait-before-tick bez catch-upu, cała pętla tempa w jednym pliku hosta.
5. **Czas hosta wyłącznie jako tempo,** nigdy jako dt. Zachować przy wprowadzaniu schedulera świata (H3).
6. **Autorytet Rust nad procesem i wejściami procesu.** React tylko renderuje.
7. **Rozdzielenie Compile (czysty) i Load (ponowna kompilacja w zaufanej granicy)** w backendzie. Zmienić warto tylko prezentację w UI.
8. **Model kotwic źródła** `SourceRef` + opcjonalny `TextSpan` i jawna konwersja skalar → UTF-16 na granicy edytora.
9. **Minimalne capabilities Tauri, ścisły CSP, `forbid(unsafe_code)`** i brak komendy `tick` w IPC.
10. **Dyscyplina CI:** pinowanie SHA, `--locked`, prawdziwy test WebView2 na hostowanym Windows, niezależny lane Linux.
11. **Brak skrótów architektonicznych typu „ocenianie w React”** lub druga symulacja instalacji w graderze. ADR-0017 słusznie tego zakazuje.

---

## 23. Questions / decisions for the technical lead

1. **Semantyka obserwacji cyklu (B1).** Czy snapshot i Variables w trybie z procesem mają pokazywać obraz wejść skanu (to, co widział PLC), wejścia przygotowane na następny skan, czy oba? Rekomenduję: trace zawsze oba; Variables domyślnie „PLC widział” z adnotacją „następny skan”.
2. **Właściciel czasu (H3).** Czy zatwierdzić ADR dla schedulera świata, w którym PLC jest uczestnikiem, z Reset Runtime niecofającym czasu świata? Jeśli nie teraz, czy akceptujesz `time_base_epoch` w trace jako rozwiązanie przejściowe?
3. **Semantyka STOP.** Czy „Stop” w PLC Lab ma pozostać „zatrzymaniem symulacji” (świat stoi, wyjścia zachowane), czy ma ewoluować w kierunku STOP-u sterownika (wyjścia w stan bezpieczny, instalacja pracuje dalej) jako profil vendor-neutral? Decyzja wpływa na kryteria bezpieczeństwa w ocenianiu.
4. **Granica silnika (H2).** Czy zatwierdzić wydzielenie bezgłowego crate’a silnika z typowanym błędem przed M5, z `desktop-session` jako adapterem IPC?
5. **Model kosztowy trace (H1).** Pełne próbki z limitem rozmiaru teraz i trace oparty na zmianach przed LAD — czy od razu trace oparty na zmianach?
6. **Strategia STL (H6).** Profil edukacyjny bez skoków (strukturalny) czy rozszerzenie IR o CFG?
7. **Los niezatwierdzonej pracy M5** w drzewie roboczym: zarchiwizować na gałęzi i przepisać na granicach z M4C, czy przejrzeć i dostosować?
8. **Strategia dystrybucji.** Podpisywanie kodu (SAC na komputerach uczniów) — kiedy wchodzi w zakres?

---

## 24. Final recommendation

Projekt ma bardzo dobre fundamenty: poprawną, deterministyczną i transakcyjną semantykę PLC, bezpieczny i ograniczony kompilator ST, poprawny kontroler wykonania ciągłego, ścisły kontrakt IPC i wiarygodne CI. Nie znalazłem błędów poprawności w rdzeniu ani wyścigów w kontrolerze. Przed ocenianiem trzeba jednak naprawić semantykę i koszt obserwacji (B1, H1), ustalić granicę silnika i ewidencji (H2), oznaczyć segmenty czasu (H3) oraz uporządkować metadane, dokumentację i repozytorium (M8, M9). Rekomenduję krótki kamień milowy M4C, a następnie M5 na stabilnej projekcji ewidencji.

AUDIT RESULT: READY WITH REQUIRED FIXES
