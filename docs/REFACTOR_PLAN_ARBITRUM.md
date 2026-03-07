# Oak Stylus Trading Engine — Refactoring Plan (Arbitrum Foundation Alignment)

**Цель:** Привести проект к требованиям Arbitrum Foundation по «feasibility» и «credible threat model»: атомарный UX по умолчанию, опциональный Commit-Reveal, фокус на институциональных ордерах, прозрачная безопасность и Public Analytics.

---

## 1. Смена парадигмы (UX & MEV)

### 1.1 Commit-Reveal опциональным, по умолчанию выключен

| Файл | Изменение |
|------|-----------|
| `src/constants.rs` | Добавить `COMMIT_REVEAL_ENABLED: bool` (или оставить только константы задержки для opt-in пути). Документировать: «Commit-Reveal опционален; основной поток — атомарный». |
| `src/state.rs` | Опционально: один `StorageBool commit_reveal_enabled` (default false), если хотим runtime toggle. Иначе достаточно документирования. |
| `src/logic.rs` | Оставить `commit_swap`, `reveal_swap`, `cancel_commitment` как есть (opt-in). Убедиться, что **основной публичный путь** для свопов — атомарный (`swap_exact_tokens_for_tokens` или единый `swap_atomic` с `min_amount_out` + `deadline`). В комментариях/NatSpec указать: «Default flow: use swap_exact_tokens_for_tokens; commit-reveal is optional for advanced users.» |
| `src/events.rs` | Без изменений (события commit/reveal остаются для opt-in). |
| `web/app/trading/page.tsx` | **По умолчанию** использовать атомарный своп: вызов `swap_exact_tokens_for_tokens(amount_in, min_amount_out, path, to, deadline)`. Опционально в UI: чекбокс «Use commit-reveal (MEV protection)» — при включении показывать текущий 4-step commit → wait → reveal. |
| `web/hooks/useOakPool.ts`, `web/lib/placeholders.ts` | Добавить/реализовать хук для атомарного свопа (`swap_exact_tokens_for_tokens`); commit-reveal оставить как опцию. |
| `web/components/LiveLogsPanel.tsx`, `web/components/SuccessModal.tsx` | Тексты: по умолчанию не «commit-reveal», а «Swap completed»; при использовании commit-reveal — «MEV-protected (commit-reveal)». |

### 1.2 Переименование проекта и позиционирование

| Файл | Изменение |
|------|-----------|
| `README.md` | Заменить «MEV-DEX» / «MEV-Protected DEX» на **«Oak Stylus Trading Engine»**. Подзаголовок: «Первый нативный Stylus-терминал с институциональными типами ордеров». Обновить badges (убрать «MEV-Protected», добавить «Stylus Trading Engine» или «Institutional Orders»). |
| `web/app/layout.tsx` | `description`: «Native Stylus trading terminal with limit orders, TP/SL and trailing stops on Arbitrum». |
| `web/package.json` | `name`: можно оставить `oak-protocol-web` или сменить на `oak-stylus-trading-engine-web`. |
| `Cargo.toml` | `name`: оставить `oak-protocol` или переименовать в `oak-stylus-trading-engine` (потребует обновления путей деплоя/артефактов). |
| `grants/APPLICATION_FULL_DRAFT.md`, `grants/ONE_PAGER.md`, `grants/README.md`, `grants/CHECKLIST.md` | Единое позиционирование: «Oak Stylus Trading Engine — первый нативный Stylus-терминал с институциональными типами ордеров (Limit, TP/SL, Trailing Stops)». Commit-reveal — опция для продвинутых пользователей; основной фокус — исполнение качества и газ. |
| `docs/ORDER_POSITION_ARCHITECTURE.md` | Обновить формулировки: MVP — атомарный своп по умолчанию; commit-reveal — опционально. |

---

## 2. Архитектурная доработка (Performance over Privacy)

### 2.1 logic.rs: убрать мульти-блочную задержку для обычных свопов

| Файл | Изменение |
|------|-----------|
| `src/logic.rs` | Обычные свопы идут только через `swap_exact_tokens_for_tokens` (и при необходимости одношаговый `swap_atomic`) — без задержки. Мульти-блочная задержка остаётся только внутри `reveal_swap` (opt-in). Никаких изменений в `process_swap` / `process_swap_from_to` для атомарного пути. |

### 2.2 Limit Orders, TP/SL, Trailing Stops — газовая оптимизация

| Файл | Изменение |
|------|-----------|
| `src/logic.rs` | Ревью `place_order`, `execute_order`, `cancel_order`, `open_position`, `close_position`, `set_position_tp_sl`, `execute_position_tp_sl`, `update_trailing_stop`, `set_position_trailing_stop`: минимизировать чтения/записи storage, убрать лишние проверки; использовать компактные типы где возможно. Документировать в NatSpec: «Gas-optimized for Stylus.» |
| `src/state.rs` | Проверить упаковку полей ордеров/позиций (например `order_type` + `status` в один слот), если ещё не сделано. |

### 2.3 Slippage Protection эффективнее Commit-Reveal

| Файл | Изменение |
|------|-----------|
| `src/logic.rs` | Единый явный путь: атомарный своп с `min_amount_out` (+ опционально `deadline`). Уже есть в `swap_exact_tokens_for_tokens` и `close_position_market`. Добавить в NatSpec/README: «Primary slippage protection: set min_amount_out (and deadline); no multi-block delay required.» |
| `README.md`, `docs/` | Описать: «Slippage protection via min_amount_out and deadline; suitable for Arbitrum execution quality.» |

---

## 3. Безопасность и публичность

### 3.1 Удалить «Internal Security Review», заменить на Testing Log

| Файл | Изменение |
|------|-----------|
| `README.md` | Удалить формулировки вида «Internal Security Review» / «comprehensive internal security audit». Заменить на: «**In-house Unit and Integration Testing**» с кратким списком: reentrancy tests, overflow tests, access control tests, order/position lifecycle, commit-reveal roundtrip. Ссылка на документ (см. ниже). |
| `SECURITY_AUDIT.md` | Переименовать или оставить; убрать любые намёки на «audit» в смысле внешнего отчёта, если такового нет. Явно указать: «Threat model and mitigations; no external audit report yet.» |
| `SECURITY_REVIEW.md` | Если файл есть и содержит «Internal Security Review» — переименовать в `TESTING_LOG.md` или включить в `docs/IN_HOUSE_TESTING.md`. Содержимое: перечень сценариев и тестов (unit/integration), без заявлений об «audit». |
| `web/AUDIT.md` | Привести в соответствие: «In-house testing and code review; no external audit.» |
| `docs/AUDIT_AND_QA_REPORT.md` | Переименовать в `docs/IN_HOUSE_TESTING_AND_QA.md` или обновить заголовок; заменить «Audit» на «In-house Unit/Integration Testing» и описать scope. |

### 3.2 Public Analytics: volume, latency, revert_rate

| Файл | Изменение |
|------|-----------|
| `src/logic.rs` | Уже есть `get_protocol_analytics` → (total_volume_token0, total_volume_token1). Добавить (или документировать) view для отчётов фонду: e.g. `get_analytics_for_report() -> (volume0, volume1, total_swaps_count?)` если нужно. Latency и revert_rate обычно считаются off-chain (indexer/backend); в контракте достаточно событий и volume. Документировать в NatSpec: «Public Analytics: volume on-chain; latency and revert_rate via indexer/events.» |
| `src/events.rs` | Убедиться, что все ключевые действия (swap, order executed, position closed, revert/cancel) эмитят события, чтобы индексер мог считать revert_rate и latency. |
| `README.md`, `docs/` | Раздел «Public Analytics»: что доступно on-chain (volume), что — через индексер (latency, revert_rate), как фонд может верифицировать. |

### 3.3 AccessControl и Timelock — 100% административных функций

| Файл | Изменение |
|------|-----------|
| `src/logic.rs` | Аудит всех админ-функций: `set_fee`, `withdraw_treasury_fees`, `set_buyback_wallet`, `clear_circuit_breaker`, `pause`, `unpause`, `set_pending_owner`, `accept_owner`. Сейчас: pause/unpause — PAUSER_ROLE; ownership — two-step + delay. Остальные — only_owner. Варианты: (1) Вынести чувствительные (set_fee, withdraw_treasury_fees, set_buyback_wallet) в Timelock (queue → delay → execute); (2) Или явно задокументировать: «Owner is governance; timelock recommended for fee/treasury.» Для 100% покрытия: либо все критичные админ-операции за Timelock, либо за ролями + документирование. |
| `src/access.rs`, `src/timelock.rs` | Без структурных изменений, если не решим добавлять вызовы через execute_operation (например, execute_set_fee(id), execute_withdraw_treasury(id)). |

---

## 4. Фронтенд-интеграция: Expected Price до отправки транзакции

| Файл | Изменение |
|------|-----------|
| `src/logic.rs` | Уже есть `get_amounts_out`, `get_current_price`. При необходимости добавить тонкий view `get_expected_output(amount_in, path) -> amount_out` (или использовать `get_amounts_out(amount_in, path)[last]`). Документировать для фронта. |
| `web/lib/oakContract.ts` | Добавить в ABI вызовы `get_amounts_out`, `get_expected_output` (если добавлен). |
| `web/app/trading/page.tsx` (или компонент свопа) | Перед отправкой транзакции: вызвать view (get_amounts_out / get_current_price для одного хопа), отобразить «Expected price» / «Expected output» и подставлять в `min_amount_out` для атомарного свопа. |

---

## 5. Сводный список файлов для изменения

- **Контракт (Rust):**  
  `src/constants.rs`, `src/state.rs` (опционально), `src/logic.rs`, `src/events.rs` (при необходимости доп. событий для аналитики).

- **Документация и брендинг:**  
  `README.md`, `SECURITY_AUDIT.md`, `SECURITY_REVIEW.md` → `docs/IN_HOUSE_TESTING.md` (или аналог), `docs/AUDIT_AND_QA_REPORT.md`, `docs/ORDER_POSITION_ARCHITECTURE.md`, `web/AUDIT.md`, `grants/APPLICATION_FULL_DRAFT.md`, `grants/ONE_PAGER.md`, `grants/README.md`, `grants/CHECKLIST.md`.

- **Фронтенд:**  
  `web/app/layout.tsx`, `web/app/trading/page.tsx`, `web/package.json`, `web/lib/oakContract.ts`, `web/hooks/useOakPool.ts`, `web/lib/placeholders.ts`, `web/components/LiveLogsPanel.tsx`, `web/components/SuccessModal.tsx`.

- **Конфиг:**  
  `Cargo.toml` (при переименовании пакета).

После выполнения плана проект будет соответствовать парадигме «atomic-by-default, commit-reveal opt-in», позиционированию «Oak Stylus Trading Engine» с институциональными ордерами, прозрачному testing log вместо заявлений об аудите и структуре Public Analytics для отчётов Arbitrum Foundation.
