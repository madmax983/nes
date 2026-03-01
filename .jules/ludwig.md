## 2026-03-01 - [Player Movement]
**Friction:** The movement speed felt too sluggish and slow, moving only 1 pixel per frame (60 pixels per second).
**Flow:** We replaced the `DEC`/`INC` movement operations with full addition and subtraction `ADC #$02` / `SBC #$02`. This increases the player movement speed by 100%, allowing for much faster, more responsive evasion of obstacles.
