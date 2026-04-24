import os
import lit.formats

config.name = "llzk-spec"
config.test_format = lit.formats.ShTest(execute_external=True)
config.suffixes = [".spec"]
config.test_source_root = os.path.dirname(__file__)
config.test_exec_root = config.test_source_root
config.substitutions.append(("%llzk_spec", os.environ.get("LLZK_SPEC_BIN", os.path.join(config.test_exec_root, "..", "..", "target", "debug", "llzk-spec"))))
