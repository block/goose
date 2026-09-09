# Status projektu: mid-turn auto-compaction

Data aktualizacji: 2026-09-09

## Cel

Naprawić automatyczne compactowanie kontekstu podczas długiej autonomicznej
pętli `model -> tool -> model`, bez dodatkowej wiadomości użytkownika i bez
rozbijania pary `tool request -> tool result`.

Docelowy PR: https://github.com/aaif-goose/goose/pull/11903

Powiązane zgłoszenie: https://github.com/aaif-goose/goose/issues/11072

## Stan bieżący

Branch roboczy: `work/mid-turn-auto-compact`.

Ostatni commit bazowy: `fc6b44f Project tool-result suffix before compaction`.

Ostatnio potwierdzony stan PR:

- PR był oznaczony jako **ready for review** na wyraźne polecenie użytkownika;
- wszystkie uruchomione joby CI zakończyły się powodzeniem;
- niektóre joby były celowo `skipped` przez konfigurację workflow;
- po tym pojawił się nowy otwarty komentarz P1, opisany w sekcji
  „Pozostałe zadania”.

W chwili sporządzania tego dokumentu nie udało się ponownie odpytać GitHub
z powodu błędu połączenia sieciowego. Powyższy status pochodzi z ostatniego
udanej weryfikacji.

## Zaimplementowane zmiany

### Obie ścieżki agenta

`AGENTS.md` wymaga zgodności legacy loop i state machine. Zmiany objęły obie:

- legacy: `crates/goose/src/agents/agent.rs`;
- state machine: `crates/goose/src/agents/state_machine/ops_compaction.rs`;
- wspólne liczenie kontekstu: `crates/goose/src/context_mgmt/mod.rs`.

### Compactowanie po tool result

- State machine dopuszcza proactive compaction po kompletnej odpowiedzi
  narzędzia, lecz nie pomiędzy requestem narzędzia a jego odpowiedzią.
- Legacy loop wykonuje kontrolę po zapisaniu tool result.
- Test regresyjny sprawdza, że historia zachowuje zgodne identyfikatory
  tool request i tool response.

### Pełniejsze liczenie kontekstu

- Do usage ostatniego inference doliczane są wiadomości dopisane po nim,
  w szczególności tool result i steer.
- Gdy request został przygotowany, liczona jest bieżąca lista narzędzi,
  prompt systemowy i wiadomości, zamiast opierania się wyłącznie na usage
  poprzedniego inference.
- Uwzględniony jest tool-shim, który przenosi schematy narzędzi do promptu.
- Sufiks tool result jest projektowany do `agent_visible_messages()`, aby
  zawartość wyłącznie dla roli User/App nie powodowała zbędnego compactowania.

### Steer i elicitation

- Legacy loop opróżnia queued steer przed kontrolą po tool result.
- Przed replacementem historii legacy reloaduje zapisaną konwersację, aby
  wymiana elicitation request/response nie została usunięta przez compaction.

## Commity

| Commit | Znaczenie |
| --- | --- |
| `ff8291b` | Pierwsza poprawka mid-turn compaction i test regresyjny. |
| `bbb71b2` | Doliczanie wiadomości po inference. |
| `21c069d` | Liczenie przygotowanego requestu przed inference. |
| `8e527f1` | Uwzględnienie queued steer w legacy check. |
| `f9716a5` | Zachowanie historii elicitation przy compaction. |
| `fc6b44f` | Projekcja tool-result suffix do agent-visible content. |

## Wykonana walidacja

Pomyślnie uruchomiono lokalnie w toku prac:

- `cargo test -p goose auto_compacts_after_a_tool_result_before_the_next_inference -- --nocapture`;
- `cargo test -p goose compaction_lifecycle`;
- `cargo fmt --check`;
- `cargo clippy -p goose --all-targets -- -D warnings`.

Wykonano też test integracyjny Ollama z `qwen3.8:27b-32k`, logicznym limitem
24 576 i progiem 50%. Compactowanie nastąpiło w trakcie pętli tool calli, po
przekroczeniu progu, a agent kontynuował pracę bez wiadomości `continue`.

Pełny CI PR był zielony po commitach wcześniejszych niż ostatnio zgłoszony P1.
Po każdej następnej poprawce należy ponownie uruchomić wymagane testy i
poczekać na CI.

Po poprawce P1 lokalnie przeszły `cargo build`, `cargo fmt --check`,
`cargo clippy -p goose --all-targets -- -D warnings` oraz test state-machine
`auto_compacts_after_a_tool_result_before_the_next_inference`. Pełne
`cargo test -p goose` uruchomione 2026-09-09 miało 8 niezwiązanych błędów
środowiskowych/bazowych: konfigurację summon i Ollama, snapshot platform
extensions, jeden test ACP oraz pięć testów JWT, dla których `jsonwebtoken`
nie może wybrać CryptoProvider. Zmiana P1 nie dotyka tych obszarów.

CI dla `97b70ea` zakończyło się zielono. Kolejna poprawka review przywraca
emit drained steer przed compaction replacement i odracza proactive
state-machine compaction po tool result do exact prepared-request hooka.
Następna poprawka review zwalnia blokadę steer po preflight, przed await
provider streamu; steer dodany potem pozostaje w kolejce dla następnego
preflight i nie zmienia już przygotowanego requestu.
Po wykryciu regresji w CI przywrócono state-machine proactive compaction dla
dużych tool resultów; legacy entry path odracza nieprecyzyjny wrapper check
po takim turnie i wykonuje exact prepared-request check przed pierwszym
provider requestem.
Najnowsza poprawka review zachowuje legacy turn budget po compaction restart
i zapisuje queued steer przed terminalnym final output.

## Pozostałe zadania

### P1: ostatni queued steer przed legacy inference

Poprawka jest zaimplementowana lokalnie i oczekuje na pełną walidację oraz
commit. Legacy path teraz:

1. legacy loop opróżnia kolejkę steer;
2. następnie wykonuje asynchroniczne operacje, w tym reload sesji oraz
   liczenie tokenów;
3. nowy steer może zostać dodany po drainie;
4. kolejna iteracja może wysłać go do providera bez ponownej kontroli progu.

1. reloaduje persisted conversation, aby zachować elicitation history;
2. pod blokadą kolejki opróżnia steer;
3. sprawdza dokładnie przygotowany request pod kątem compaction;
4. rozpoczyna `stream_response_from_provider` przed zwolnieniem tej blokady.

Zdarzenia steer są emitowane po rozpoczęciu streamu, aby nie zawieszać
generatora na kliencie próbującym równocześnie dodać steer.

Należy jeszcze dodać deterministyczny test legacy path, w którym duży steer
jest dodawany po pierwszym drainie i przed kolejnym inference. Test powinien
dowieść, że compaction następuje przed provider call.

### Parzystość ścieżek

Każdą zmianę legacy loop należy ocenić pod kątem state machine. State machine
nie ma analogicznego przejścia: `SteerOperation` po drainie zwraca osobny
efekt, więc następny cykl ponownie wykonuje `CompactionOperation` i
`PreparedRequestCompactionHook` przed inference. Steer dodany później zostaje
w kolejce dla kolejnego cyklu i nie jest dodawany do bieżącego requestu.

### Wymagania workflow upstream

Repozytoryjny `AGENTS.md` wymaga, aby issue implementowane w zewnętrznym PR
miało status **Ready** na Goose Issues board. Ostatni odczyt #11072 zwrócił
brak przypisania do boardu (`projectItems: []`). Przed dalszym upstreamowym
wdrażaniem należy potwierdzić u maintainera status Ready albo uzyskać jego
wyraźną dyspozycję traktującą pracę jako maintainer-directed.

## Checklista przed merge

- [x] Naprawić P1 dotyczący późnego queued steer (lokalnie, przed commitem).
- [ ] Dodać test legacy odtwarzający ten przypadek.
- [ ] Potwierdzić parzystość z state machine.
- [ ] Uruchomić `source bin/activate-hermit && cargo build`.
- [ ] Uruchomić `cargo test -p goose`.
- [ ] Uruchomić `cargo fmt --check`.
- [ ] Uruchomić `cargo clippy --all-targets -- -D warnings`.
- [ ] Poczekać na zielone CI dla ostatniego commitu.
- [ ] Sprawdzić i zamknąć wszystkie wątki review.
- [ ] Potwierdzić status Ready issue #11072 lub uzyskać dyspozycję maintainera.
