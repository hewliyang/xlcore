import json, re, pathlib
root = pathlib.Path(__file__).resolve().parent.parent

# 1. Reference = "all Excel functions": ECMA-376 Part 1 (§18.17.7) ∪ EPPlus registry
ecma_json = json.load(open(root/'ecma-376/parse_result_2026-05-17.json'))
content = ecma_json['result']['chunks'][0]['content']
ecma = set(re.findall(r'18\.17\.7\.\d+\s+([A-Z][A-Z0-9_.]*)\b', content))

builtins = (root/'epplus/src/EPPlus/FormulaParsing/Excel/Functions/BuiltInFunctions.cs').read_text()
epplus = set(m.upper() for m in re.findall(r'Functions\["([a-z0-9_.]+)"\]\s*=', builtins))

reference = (ecma | epplus) - {'ANCHORARRAY', 'SINGLE'}  # internal @ helpers

# 2. ironcalc implemented set (english names + SUMPRODUCT special-case)
lang = json.load(open(root/'crates/ironcalc-base/src/language/language.json'))
iron = set(str(v).upper() for v in lang['en']['functions'].values()) | {'SUMPRODUCT'}

missing = sorted(reference - iron)
print(f'reference={len(reference)} ironcalc={len(iron)} missing={len(missing)}')
print('\n'.join(missing))
