import re

with open('crates/nes-core/src/api.rs', 'r') as f:
    content = f.read()

content = re.sub(r'MapperDeltaKind::Mmc3\(state\) => \{\n\s+let Self::Mmc3\(mapper\) = self else \{\n\s+debug_assert!\(false, "mapper delta kind must match mapper variant"\);\n\s+return;\n\s+\};\n\s+mapper\.restore_state\(state\.clone\(\)\);', 'MapperDeltaKind::Mmc3(state) => {\n                let Self::Mmc3(mapper) = self else {\n                    debug_assert!(false, "mapper delta kind must match mapper variant");\n                    return;\n                };\n                mapper.restore_state(state);', content)

content = re.sub(r'MapperDeltaKind::Fme7\(state\) => \{\n\s+let Self::Fme7\(mapper\) = self else \{\n\s+debug_assert!\(false, "mapper delta kind must match mapper variant"\);\n\s+return;\n\s+\};\n\s+mapper\.restore_state\(state\.clone\(\)\);', 'MapperDeltaKind::Fme7(state) => {\n                let Self::Fme7(mapper) = self else {\n                    debug_assert!(false, "mapper delta kind must match mapper variant");\n                    return;\n                };\n                mapper.restore_state(state);', content)

content = re.sub(r'MapperDeltaKind::Mmc4\(state\) => \{\n\s+let Self::Mmc4\(mapper\) = self else \{\n\s+debug_assert!\(false, "mapper delta kind must match mapper variant"\);\n\s+return;\n\s+\};\n\s+mapper\.restore_state\(state\.clone\(\)\);', 'MapperDeltaKind::Mmc4(state) => {\n                let Self::Mmc4(mapper) = self else {\n                    debug_assert!(false, "mapper delta kind must match mapper variant");\n                    return;\n                };\n                mapper.restore_state(state);', content)

content = re.sub(r'MapperDeltaKind::Mmc5\(state\) => \{\n\s+let Self::Mmc5\(mapper\) = self else \{\n\s+debug_assert!\(false, "mapper delta kind must match mapper variant"\);\n\s+return;\n\s+\};\n\s+mapper\.restore_state\(state\.clone\(\)\);', 'MapperDeltaKind::Mmc5(state) => {\n                let Self::Mmc5(mapper) = self else {\n                    debug_assert!(false, "mapper delta kind must match mapper variant");\n                    return;\n                };\n                mapper.restore_state(state);', content)

with open('crates/nes-core/src/api.rs', 'w') as f:
    f.write(content)
