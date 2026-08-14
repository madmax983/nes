import re

with open("crates/nes-core/src/api.rs", "r") as f:
    api_content = f.read()

# Make sure they are gone:
api_content = api_content.replace('self.ports.controller_port_sample(Player::One, CONTROLLER_OPEN_BUS_MASK)', 'self.ports.controller_port_sample(Player::One)')
api_content = api_content.replace('self.ports.controller_port_sample(Player::Two, CONTROLLER_OPEN_BUS_MASK)', 'self.ports.controller_port_sample(Player::Two)')

with open("crates/nes-core/src/api.rs", "w") as f:
    f.write(api_content)
