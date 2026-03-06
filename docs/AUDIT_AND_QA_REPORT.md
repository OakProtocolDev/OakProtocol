# Oak Protocol — Audit & QA Report (Senior Auditor)

## Критические находки и исправления

### 1. **Pausable не применялся к close_position и execute_position_tp_sl** — ИСПРАВЛЕНО
- **Риск:** При включённой паузе контракта пользователи и киперы могли продолжать закрывать позиции и вызывать TP/SL, что противоречит ожиданию «критические функции блокируются при паузе».
- **Исправление:** В начале `close_position` добавлен вызов `require_not_paused(self)?`. В начале `execute_position_tp_sl` добавлен вызов `require_not_paused(self)?`.

### 2. **Reentrancy в close_position / update_trailing_stop**
- **Проверка:** Оба пути защищены: в начале вызывается `lock_reentrancy_guard(self)?`, внешние вызовы (`safe_transfer`, `process_swap_from_to`) выполняются после обновления состояния (margin balance, position_status), в конце вызывается `unlock_reentrancy_guard(self)`.
- **Вердикт:** CEI соблюдён, reentrancy-риск закрыт.

### 3. **Арифметика в close_position и execute_position_tp_sl**
- **Проверка:** Используются `checked_add`, `checked_sub` для `margin_total` и обновления `position_margin_balance`. Деление в `get_current_price` защищено проверкой `reserve_out.is_zero()`.
- **Вердикт:** Явных неконтролируемых переполнений не выявлено.

---

## Storage Layout (sol_storage!)

- **Коллизии:** Структуры `OakDEX` и `OakSentinel` используют `sol_storage!` с последовательным выделением слотов. Отдельные поля и вложенные `StorageMap` не пересекаются; коллизий слотов не обнаружено.
- **Рекомендация:** При добавлении новых полей добавлять их только в конец структур (после `reserved3` / `sentinel_reserved*`), чтобы не менять слоты существующих полей.

---

## Gas

- **Лишние чтения:** В путях position (close / TP-SL) многократно вызываются `self.position_* .setter(key).get()` для одного и того же `key`. Объединение в один блок чтений (локальные переменные) уже частично есть; при рефакторинге можно вынести все поля позиции в один проход для минимизации SLOAD.
- **Роли и таймлок:** Обращения к `roles` и `timelock_ready_block` — по одному чтению/записи на операцию; значимой оптимизации без изменения логики не требуется.

---

## Clippy

- Исправлено: `obfuscated_if_else` и `unnecessary_lazy_evaluations` в расчёте `impact_bps` (logic.rs).
- Исправлено: `useless_conversion` в timelock.rs и logic.rs (`enc_u256`).
- Исправлено: `needless_borrows_for_generic_args` в rng.rs.
- Исправлено: неиспользуемые параметры в vault.rs (`_index_token`).
- Оставлены без изменений: предупреждения о «never used» для функций/констант, используемых только в `#[cfg(all(not(test), target_arch = "wasm32"))]` (публичный impl OakDEX), так как в тестах этот блок не компилируется.

---

## Функции по блокам, требующие дополнительной проверки

| Блок      | Функции | Что проверить |
|----------|---------|-------------------------------|
| Trading  | `reveal_swap`, `process_swap`, `swap_exact_tokens_for_tokens` | Дедлайн и slippage; корректность пула и резервов при мульти-хупе. |
| Risk     | `close_position`, `execute_position_tp_sl`, `update_trailing_stop` | Пауза (сделано), reentrancy (проверено), арифметика (проверено). Дополнительно: поведение при нулевой ликвидности пула при закрытии. |
| Social   | Orders: `place_order`, `execute_order`, `cancel_order` | Проверка владельца, статуса и условия срабатывания; OCO-отмена. |
| Infra    | `withdraw_treasury_fees`, `set_fee`, ownership transfer | Резервы пула не должны включать treasury/buyback; задержка смены владельца. |
| Security | `pause`/`unpause` (Pausable + AccessControl), `grant_role`/`revoke_role`, Timelock `queue_operation`/`execute_operation` | Роли только у multisig; таймлок: минимальная задержка и однократное выполнение по id. |

---

## Тесты

- Добавлен `tests/protocol_test.rs`: сценарии Happy Path (swap → позиция → TP/SL по логике), Error Cases (чужая позиция, недостаточная ликвидность, вызов при паузе), Edge Cases (slippage при SL, таймлок).
- **Важно:** При сборке тестов (`cargo test`) линковка падает из‑за `native_keccak256` (символ окружения Stylus). Запуск интеграционных тестов возможен в Stylus test env или при наличии стаба для `native_keccak256` при native-сборке.
- Для полного E2E с **Foundry** добавлен пример в `tests/foundry/README_FOUNDRY_TESTS.md` и заглушка теста Solidity.
