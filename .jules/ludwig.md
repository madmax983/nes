## 2026-03-05 - Player Movement
**Friction:** The movement felt very sluggish when moving the player. The 1 pixel per frame speed was not responsive enough.
**Flow:** We fixed the feel by increasing the movement speed to $02 pixels per frame via the `PLAYER_SPEED` constant. Movement logic was changed from `inc`/`dec` to `adc`/`sbc`.
