# Fingering review — round-8 winner, after the 2026-10-29 answers

layout `;tspbxgoiwndhrlyecuafkvmzjq,./`  ·  model variant A (full-span M/R & R/P reverse scissors impossible)
- Cross-hand bigrams (no fingering decision): **68.8%** of bigram mass
- Same-hand bigrams: **28.4%**; geometric model and current model pick the same fingering on **87.9%** of that mass
## 1. Top 40 same-hand bigrams

| bigram | freq % | keys | geo pick | cost | runner-up (Δ) | current-model pick | cost | why (geo cost parts) |
|---|---|---|---|---|---|---|---|---|
| th | 3.75 | W→D | **RM** | 0.40 | RI (+0.05) | RM | 2.05 | {'vert': 0.43, 'roll': -0.23, 'key_b': 0.19} |
| nd | 1.59 | A→S | **PR** | 0.57 | RM (+0.23) | PR | 2.25 | {'vert': 0.32, 'roll': 0.06, 'key_b': 0.19} |
| ou | 1.06 | I→L | **MR** | 0.06 | MM (+6.73) | MR | 1.99 | {'roll': -0.13, 'key_b': 0.19} |
| st | 0.97 | E→W | **MR** | -0.20 | RR (+5.97) | MR | 1.80 | {'roll': -0.2} |
| nt | 0.94 | A→W | **PR** | 0.06 | RR (+6.80) | PR | 2.06 | {'roll': 0.06} |
| co | 0.73 | K→I | **IM** | 1.56 | RM (+0.41) | IM | 1.97 | {'cross': 1.62, 'roll': -0.28, 'key_a': 0.3} |
| ea | 0.69 | J→; | **IP** | 0.50 | IR (+0.61) | IP | 2.50 | {'leap': 0.5} |
| ce | 0.64 | K→J | **MI** | 0.28 | RM (+0.52) | MI | 1.96 | {'vert': 0.37, 'roll': -0.28, 'key_a': 0.19} |
| io | 0.64 | O→I | **RM** | -0.35 | MM (+6.12) | RM | 1.65 | {'roll': -0.35} |
| ic | 0.57 | O→K | **RM** | 0.59 | MI (+0.08) | RM | 2.05 | {'vert': 0.43, 'cross': 0.32, 'roll': -0.35, 'key_b': 0.19} |
| wa | 0.51 | P→; | **RP** | 1.49 | MR (+2.12) | RP | 2.06 | {'cross': 1.43, 'roll': -0.14, 'key_a': 0.2} |
| ca | 0.45 | K→; | **MP** | 0.62 | RP (+0.05) | RP ✗ | 2.28 | {'vert': 0.13, 'leap': 0.3, 'key_a': 0.19} |
| sh | 0.44 | E→D | **MI** | 1.37 | MM (+5.12) | MI | 1.97 | {'cross': 1.43, 'roll': -0.28, 'key_b': 0.3} |
| wi | 0.43 | P→O | **RM** | 0.58 | RR (+4.62) | RM | 2.09 | {'roll': -0.35, 'key_a': 0.2, 'key_b': 0.77} |
| ns | 0.43 | A→E | **PM** | 0.30 | RM (+0.48) | PR ✗ | 2.28 | {'leap': 0.3} |
| rs | 0.42 | F→E | **IM** | -0.18 | IR (+1.10) | IM | 1.73 | {'roll': -0.18} |
| ow | 0.42 | I→P | **MR** | 0.15 | RR (+5.82) | MR | 2.10 | {'roll': -0.05, 'key_b': 0.2} |
| pr | 0.41 | R→F | **MI** | 1.36 | II (+5.24) | MI | 1.93 | {'cross': 1.43, 'roll': -0.28, 'key_a': 0.2} |
| ac | 0.41 | ;→K | **PM** | 0.62 | PR (+0.23) | PR ✗ | 2.38 | {'vert': 0.13, 'leap': 0.3, 'key_b': 0.19} |
| ai | 0.40 | ;→O | **PR** | 0.04 | PM (+1.03) | PR | 2.06 | {} |
| ie | 0.39 | O→J | **RI** | 0.15 | MI (+0.55) | MI ✗ | 2.10 | {'leap': 0.15} |
| we | 0.38 | P→J | **RI** | 0.35 | RM (+1.69) | RI | 2.35 | {'leap': 0.15, 'key_a': 0.2} |
| ec | 0.37 | J→K | **IM** | 0.28 | IR (+0.57) | IM | 1.96 | {'vert': 0.37, 'roll': -0.28, 'key_b': 0.19} |
| tr | 0.35 | W→F | **RI** | 0.15 | RM (+1.58) | RI | 2.15 | {'leap': 0.15} |
| ge | 0.33 | U→J | **MI** | 1.55 | IM (+1.38) | MI | 1.93 | {'cross': 1.62, 'roll': -0.28, 'key_a': 0.2} |
| rt | 0.30 | F→W | **IR** | 0.15 | MR (+1.36) | IR | 2.15 | {'leap': 0.15} |
| ts | 0.29 | W→E | **RM** | -0.35 | RR (+6.12) | RM | 1.65 | {'roll': -0.35} |
| ld | 0.28 | G→S | **IR** | 0.54 | IM (+0.45) | IR | 2.54 | {'leap': 0.15, 'key_a': 0.2, 'key_b': 0.19} |
| fr | 0.28 | Z→F | **PI** | 0.74 | RI (+0.33) | PI | 2.74 | {'leap': 0.5, 'key_a': 0.24} |
| ay | 0.26 | ;→H | **PI** | 0.70 | RI (+0.61) | PI | 2.70 | {'leap': 0.5, 'key_b': 0.2} |
| ig | 0.23 | O→U | **RM** | 0.11 | RI (+0.33) | RM | 2.03 | {'roll': -0.09, 'key_b': 0.2} |
| yo | 0.23 | H→I | **IM** | 0.14 | IR (+0.95) | IM | 2.06 | {'roll': -0.06, 'key_a': 0.2} |
| pl | 0.23 | R→G | **MI** | 0.19 | RI (+0.63) | MI | 2.12 | {'roll': -0.18, 'key_a': 0.2, 'key_b': 0.2} |
| ia | 0.22 | O→; | **RP** | -0.09 | MP (+1.16) | RP | 1.86 | {'roll': -0.09} |
| bl | 0.22 | T→G | **MI** | 1.72 | II (+5.22) | MI | 2.32 | {'cross': 1.43, 'roll': -0.28, 'key_a': 0.4, 'key_b': 0.2} |
| mp | 0.21 | V→R | **IM** | 1.57 | II (+6.97) | IM | 2.25 | {'vert': 0.68, 'cross': 0.48, 'key_a': 0.24, 'key_b': 0.2} |
| wo | 0.20 | P→I | **RM** | 0.11 | RR (+5.86) | RM | 2.03 | {'roll': -0.09, 'key_a': 0.2} |
| ci | 0.19 | K→O | **MR** | 0.62 | IM (+0.05) | MR | 2.20 | {'vert': 0.31, 'cross': 0.32, 'roll': -0.2, 'key_a': 0.19} |
| ag | 0.19 | ;→U | **PM** | 0.50 | PI (+0.29) | RM ✗ | 2.43 | {'leap': 0.3, 'key_b': 0.2} |
| rd | 0.18 | F→S | **IR** | 0.34 | IM (+0.34) | IM ✗ | 2.25 | {'leap': 0.15, 'key_b': 0.19} |

## 2. Disagreements with the current model, by frequency (top 40 of 56)

| bigram | freq % | keys | geo pick | cost | runner-up (Δ) | current-model pick | cost | why (geo cost parts) |
|---|---|---|---|---|---|---|---|---|
| ca | 0.45 | K→; | **MP** | 0.62 | RP (+0.05) | RP ✗ | 2.28 | {'vert': 0.13, 'leap': 0.3, 'key_a': 0.19} |
| ns | 0.43 | A→E | **PM** | 0.30 | RM (+0.48) | PR ✗ | 2.28 | {'leap': 0.3} |
| ac | 0.41 | ;→K | **PM** | 0.62 | PR (+0.23) | PR ✗ | 2.38 | {'vert': 0.13, 'leap': 0.3, 'key_b': 0.19} |
| ie | 0.39 | O→J | **RI** | 0.15 | MI (+0.55) | MI ✗ | 2.10 | {'leap': 0.15} |
| ag | 0.19 | ;→U | **PM** | 0.50 | PI (+0.29) | RM ✗ | 2.43 | {'leap': 0.3, 'key_b': 0.2} |
| rd | 0.18 | F→S | **IR** | 0.34 | IM (+0.34) | IM ✗ | 2.25 | {'leap': 0.15, 'key_b': 0.19} |
| ex | 0.18 | J→Y | **MI** | 2.48 | II (+3.99) | IM ✗ | 2.63 | {'vert': 1.47, 'roll': -0.18, 'key_a': 0.86, 'key_b': 0.47} |
| ei | 0.17 | J→O | **IR** | 0.15 | IM (+0.55) | IM ✗ | 2.10 | {'leap': 0.15} |
| go | 0.14 | U→I | **IM** | 0.29 | MR (+0.45) | MR ✗ | 2.06 | {'vert': 0.37, 'roll': -0.28, 'key_a': 0.29} |
| dr | 0.14 | S→F | **RI** | 0.34 | MI (+0.34) | MI ✗ | 2.25 | {'leap': 0.15, 'key_a': 0.19} |
| br | 0.12 | T→F | **IM** | 2.87 | MI (+0.82) | MI ✗ | 2.43 | {'vert': 1.47, 'cross': 0.48, 'roll': -0.28, 'key_a': 0.44, 'key_b': 0.96} |
| ui | 0.08 | L→O | **IM** | 2.58 | RM (+0.05) | RM ✗ | 2.08 | {'cross': 1.62, 'roll': -0.28, 'key_a': 0.5, 'key_b': 0.77} |
| kn | 0.08 | X→A | **IR** | 1.45 | MR (+0.16) | RP ✗ | 2.66 | {'leap': 0.15, 'key_a': 0.54, 'key_b': 0.86} |
| nk | 0.07 | A→X | **RI** | 1.45 | RM (+0.29) | PR ✗ | 2.76 | {'leap': 0.15, 'key_a': 0.86, 'key_b': 0.54} |
| ph | 0.06 | R→D | **MR** | 0.95 | IM (+1.11) | MI ✗ | 2.47 | {'cross': 0.1, 'roll': -0.2, 'key_a': 0.2, 'key_b': 0.98} |
| oa | 0.06 | I→; | **MP** | 0.30 | RP (+0.47) | RP ✗ | 1.99 | {'leap': 0.3} |
| rv | 0.06 | F→C | **IM** | 2.59 | MI (+1.20) | MI ✗ | 2.56 | {'vert': 1.47, 'cross': 0.95, 'roll': -0.28, 'key_b': 0.53} |
| og | 0.05 | I→U | **MI** | 0.29 | RM (+0.30) | RM ✗ | 1.91 | {'vert': 0.37, 'roll': -0.28, 'key_b': 0.29} |
| xi | 0.03 | Y→O | **IR** | 0.62 | MR (+2.78) | MR ✗ | 2.30 | {'leap': 0.15, 'key_a': 0.47} |
| xe | 0.02 | Y→J | **IM** | 2.48 | II (+4.29) | MI ✗ | 2.63 | {'vert': 1.47, 'roll': -0.18, 'key_a': 0.47, 'key_b': 0.86} |
| ix | 0.02 | O→Y | **RI** | 0.62 | RM (+2.78) | RM ✗ | 2.23 | {'leap': 0.15, 'key_b': 0.47} |
| rb | 0.02 | F→T | **MI** | 2.87 | IM (+0.82) | IM ✗ | 2.43 | {'vert': 1.47, 'cross': 0.48, 'roll': -0.28, 'key_a': 0.96, 'key_b': 0.44} |
| bt | 0.02 | T→W | **IR** | 0.59 | MR (+2.81) | MR ✗ | 2.30 | {'leap': 0.15, 'key_a': 0.44} |
| sf | 0.02 | E→Z | **MP** | 0.54 | MR (+0.38) | RP ✗ | 2.42 | {'leap': 0.3, 'key_b': 0.24} |
| sn | 0.01 | E→A | **MP** | 0.30 | RP (+0.44) | RP ✗ | 2.18 | {'leap': 0.3} |
| ax | 0.01 | ;→Y | **PM** | 0.70 | PI (+0.27) | RM ✗ | 2.62 | {'leap': 0.3, 'key_b': 0.4} |
| fs | 0.01 | Z→E | **PM** | 0.54 | RM (+0.38) | PR ✗ | 2.52 | {'leap': 0.3, 'key_a': 0.24} |
| hb | 0.00 | D→T | **RM** | 1.15 | RI (+0.41) | MI ✗ | 5.38 | {'roll': -0.08, 'key_a': 0.98, 'key_b': 0.4} |
| tz | 0.00 | W→B | **RI** | 1.60 | RM (+11.33) | RM ✗ | 8.73 | {'vert': 1.01, 'leap': 0.15, 'key_b': 0.44} |
| np | 0.00 | A→R | **PM** | 0.50 | PI (+0.29) | PR ✗ | 2.46 | {'leap': 0.3, 'key_b': 0.2} |
| iq | 0.00 | O→M | **RI** | 1.41 | MI (+0.28) | MI ✗ | 2.64 | {'vert': 1.01, 'leap': 0.15, 'key_b': 0.24} |
| ao | 0.00 | ;→I | **PM** | 0.30 | PR (+0.47) | PR ✗ | 2.09 | {'leap': 0.3} |
| bv | 0.00 | T→C | **IM** | 3.99 | MI (+2.95) | MI ✗ | 3.21 | {'vert': 3.31, 'key_a': 0.44, 'key_b': 0.53} |
| xu | 0.00 | Y→L | **IR** | 0.81 | MR (+5.74) | MR ✗ | 2.49 | {'leap': 0.15, 'key_a': 0.47, 'key_b': 0.19} |
| vr | 0.00 | C→F | **MI** | 2.59 | IM (+1.20) | IM ✗ | 2.56 | {'vert': 1.47, 'cross': 0.95, 'roll': -0.28, 'key_a': 0.53} |
| ux | 0.00 | L→Y | **RI** | 0.81 | RM (+5.74) | RM ✗ | 2.42 | {'leap': 0.15, 'key_a': 0.19, 'key_b': 0.47} |
| zl | 0.00 | B→G | **II** | 6.64 | MI (+6.49) | MI ✗ | 4.25 | {'key_a': 0.44, 'key_b': 0.2} |
| dt | 0.00 | S→W | **IM** | 2.39 | RR (+3.80) | MI ✗ | 4.60 | {'cross': 1.43, 'roll': -0.28, 'key_a': 0.5, 'key_b': 0.77} |
| pn | 0.00 | R→A | **MP** | 0.50 | IP (+0.29) | RP ✗ | 2.36 | {'leap': 0.3, 'key_a': 0.2} |
| tb | 0.00 | W→T | **RI** | 0.59 | RM (+2.81) | RM ✗ | 2.23 | {'leap': 0.15, 'key_b': 0.44} |

## 3. Closest calls among the 120 most common same-hand bigrams (runner-up within 0.20) — best questions to ask you

| bigram | freq % | keys | geo pick | cost | runner-up (Δ) | current-model pick | cost | why (geo cost parts) |
|---|---|---|---|---|---|---|---|---|
| th | 3.75 | W→D | **RM** | 0.40 | RI (+0.05) | RM | 2.05 | {'vert': 0.43, 'roll': -0.23, 'key_b': 0.19} |
| ic | 0.57 | O→K | **RM** | 0.59 | MI (+0.08) | RM | 2.05 | {'vert': 0.43, 'cross': 0.32, 'roll': -0.35, 'key_b': 0.19} |
| ca | 0.45 | K→; | **MP** | 0.62 | RP (+0.05) | RP ✗ | 2.28 | {'vert': 0.13, 'leap': 0.3, 'key_a': 0.19} |
| ci | 0.19 | K→O | **MR** | 0.62 | IM (+0.05) | MR | 2.20 | {'vert': 0.31, 'cross': 0.32, 'roll': -0.2, 'key_a': 0.19} |
| ht | 0.17 | D→W | **MR** | 0.37 | IR (+0.08) | MR | 2.20 | {'vert': 0.31, 'roll': -0.13, 'key_a': 0.19} |
| qu | 0.11 | M→L | **IR** | 0.58 | IM (+0.02) | IR | 2.58 | {'leap': 0.15, 'key_a': 0.24, 'key_b': 0.19} |
| ui | 0.08 | L→O | **IM** | 2.58 | RM (+0.05) | RM ✗ | 2.08 | {'cross': 1.62, 'roll': -0.28, 'key_a': 0.5, 'key_b': 0.77} |
| kn | 0.08 | X→A | **IR** | 1.45 | MR (+0.16) | RP ✗ | 2.66 | {'leap': 0.15, 'key_a': 0.54, 'key_b': 0.86} |
| ju | 0.06 | N→L | **IR** | 0.58 | IM (+0.19) | IR | 2.58 | {'leap': 0.15, 'key_a': 0.24, 'key_b': 0.19} |
| cy | 0.04 | K→H | **MI** | 0.69 | RI (+0.04) | MI | 2.30 | {'vert': 0.37, 'roll': -0.07, 'key_a': 0.19, 'key_b': 0.2} |

## 4. One-hand trigrams in the top 200

| trigram | freq % | keys | geo pick | cost | runner-up | current-model pick | cost |
|---|---|---|---|---|---|---|---|
| you | 0.60 | H→I→L | **IMR** | 0.10 | IRR (+7.48) | IMR | 3.20 |
| cou | 0.40 | K→I→L | **IMR** | 1.67 | RMR (+3.16) | IMR | 2.85 |
| str | 0.39 | E→W→F | **MRI** | 0.25 | MRM (+5.08) | MRI | 3.25 |
| ica | 0.35 | O→K→; | **MIR** | 2.04 | RMP (+0.78) | MIR | 4.05 |
| oug | 0.30 | I→L→U | **MRI** | 0.98 | MRM (+2.08) | MRI | 3.77 |
| ace | 0.30 | ;→K→J | **PMI** | 0.61 | RMI (+0.44) | PMI | 3.16 |
| ice | 0.26 | O→K→J | **RMI** | 0.58 | MII (+5.09) | RMI | 2.72 |
| age | 0.24 | ;→U→J | **PMI** | 1.75 | PIM (+3.54) | PMI | 3.12 |
