# Foundry E2E tests for Oak Protocol (Stylus)

Контракт собран в WASM (Rust/Stylus) и деплоится на Arbitrum. Для E2E в Foundry нужны:

1. **Скомпилированный и задеплоенный Stylus-контракт** (адрес в тестах).
2. **Solidity-интерфейс** к внешним методам (см. `IOakProtocol.sol`).
3. **Форк Arbitrum** или тестовая сеть, где развёрнут контракт.

## Пример сценариев для Forge

- **Happy path:** `init` → `create_pool` → `add_liquidity` → `commit_swap` → (advance blocks) → `reveal_swap` → `open_position` → `set_position_tp_sl` → (изменить резервы/цену) → `execute_position_tp_sl` → проверка балансов.
- **Error:** Вызов `close_position(positionId, minOut)` от имени не-владельца → ожидание `revert` с сообщением `POSITION_NOT_OWNER`.
- **Error:** Вызов `reveal_swap` при `paused() == true` → ожидание `revert` с сообщением `PAUSED`.
- **Edge:** Вызов `execute_position_tp_sl` с очень большим `min_amount_out` (slippage) → ожидание `revert` с сообщением `SLIPPAGE_EXCEEDED`.
- **Timelock:** `queue_operation` от TIMELOCK_ADMIN → (advance blocks >= delay) → `execute_operation` с теми же параметрами → успех.

Файл `OakProtocol.t.sol` — заглушка; подставьте свой адрес контракта и раскомментируйте вызовы после деплоя.
