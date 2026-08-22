# Examples

None of these schemas ship with bottle. They show how to
declare a type and use the verbs. Copy the YAML, change the
fields.

Set `BOTTLE_AGENT`. Sample timestamps are local to the
host. A link target is `schema/id`. Link names are chosen
at write time; they are not in the YAML.

A caller may turn a shorthand like `4x8x24` into several
`log`s (or one MCP `entries` call). bottle only sees fields
and links.

## Meals

Meals and drinks are different types so `sum ml` and
`sum protein` stay obvious.

`nutrition.meal.yaml`

```yaml
fields:
  - name: when
    type: enum
    required: true
    values: [breakfast, snack, lunch, dinner, extra]
  - name: what
    type: text
    required: true
  - name: kcal
    type: number
    required: true
  - name: protein
    type: number
    required: true
  - name: carbs
    type: number
    required: true
  - name: fat
    type: number
    required: false
```

`nutrition.fluid.yaml`

```yaml
fields:
  - name: ml
    type: number
    required: true
  - name: kind
    type: enum
    required: true
    values: [water, soda, other]
```

```
bottle schema add nutrition.meal --file nutrition.meal.yaml
bottle schema add nutrition.fluid --file nutrition.fluid.yaml
bottle schema show nutrition.meal
bottle log nutrition.meal when=breakfast what="4 eggs" \
  kcal=568 protein=49 carbs=5 fat=39.6
bottle log nutrition.fluid ml=380 kind=water
bottle today nutrition.meal
bottle sum nutrition.meal protein --from 2026-08-16 \
  --to 2026-08-22 --group day
bottle amend nutrition.fluid 1 ml=375
bottle ignore nutrition.fluid 1
```

`get`, `amend`, and `ignore` take a schema because ids are
per table. `today` does not print a total.

## Sets

One entry is one set. `reps`, `load`, and `unit` stay
queryable. Volume is `reps * load` when load is present, so
`sum volume` is tonnage. How many sets is an entry count.

`4x8x24kg` is four entries of `reps=8 load=24 unit=kg`, not
one entry with a sets count. If the last set is 6, the four
entries differ.

`load` is a number. `unit` is `kg` or `lb`. If `load` is
set, `unit` is required. Bodyweight sets omit both. bottle
does not convert units. `sum volume` should use
`--where unit=kg` (or `lb`).

A workout is its own entry. Sets link to it as `session`.

`fitness.session.yaml`

```yaml
fields:
  - name: title
    type: text
    required: false
```

`fitness.set.yaml`

```yaml
fields:
  - name: movement
    type: text
    required: true
  - name: reps
    type: number
    required: true
  - name: load
    type: number
    required: false
  - name: unit
    type: enum
    required: false
    values: [kg, lb]
  - name: volume
    type: number
    required: false
```

```
bottle schema add fitness.session \
  --file fitness.session.yaml
bottle schema add fitness.set --file fitness.set.yaml
bottle log fitness.session title="upper"
bottle log fitness.set --link session=fitness.session/1 \
  movement=squat reps=8 load=24 unit=kg
bottle log fitness.set --link session=fitness.session/1 \
  movement=squat reps=8 load=24 unit=kg
bottle ls fitness.set --where session=fitness.session/1
bottle sum fitness.set volume --from 2026-08-16 \
  --to 2026-08-22 --where unit=kg
```

A single set with no workout omits `--link`.

## Cardio

A cardio bout is one entry (duration and kind), not a list
of intervals.

`fitness.cardio.yaml`

```yaml
fields:
  - name: protocol
    type: enum
    required: true
    values: [easy, intervals, warmup, other]
  - name: modality
    type: enum
    required: true
    values: [bike, row, run, other]
  - name: minutes
    type: number
    required: true
```

```
bottle schema add fitness.cardio --file fitness.cardio.yaml
bottle log fitness.cardio protocol=easy modality=bike \
  minutes=30
bottle log fitness.cardio protocol=intervals \
  modality=bike minutes=40
bottle log fitness.cardio --link session=fitness.session/1 \
  protocol=warmup modality=bike minutes=5
bottle last fitness.cardio --where protocol=intervals
```

## Expenses

`kind` is `in` or `out`. Sum spend with
`--where kind=out`. Do not store a negative amount.

`money.txn.yaml`

```yaml
fields:
  - name: account
    type: text
    required: true
  - name: amount
    type: number
    required: true
  - name: kind
    type: enum
    required: true
    values: [in, out]
  - name: note
    type: text
    required: false
```

```
bottle schema add money.txn --file money.txn.yaml
bottle log money.txn account=operating amount=2400 \
  kind=in note="invoice 1841"
bottle log money.txn account=operating amount=86.50 \
  kind=out note=software
bottle sum money.txn amount --from 2026-08-01 \
  --to 2026-08-31 --where kind=out
bottle sum money.txn amount --from 2025-01-01 \
  --to 2026-12-31 --where kind=out --group month
bottle last money.txn --where account=operating
```

## Hours

A project is its own table. Hour entries link to it as
`project`.

`work.project.yaml`

```yaml
fields:
  - name: name
    type: text
    required: true
```

`work.hours.yaml`

```yaml
fields:
  - name: hours
    type: number
    required: true
  - name: note
    type: text
    required: false
```

```
bottle schema add work.project --file work.project.yaml
bottle schema add work.hours --file work.hours.yaml
bottle log work.project name=acme
bottle log work.hours --link project=work.project/1 \
  hours=2.5 note=api
bottle today work.hours
bottle sum work.hours hours --from 2026-08-16 \
  --to 2026-08-22 --where project=work.project/1
bottle sum work.hours hours --from 2026-08-01 \
  --to 2026-08-31 --group project
```

## Follow-ups

Use `last` to see when you last spoke to someone.

`crm.touch.yaml`

```yaml
fields:
  - name: who
    type: text
    required: true
  - name: channel
    type: enum
    required: true
    values: [email, call, meet, other]
  - name: note
    type: text
    required: false
```

```
bottle schema add crm.touch --file crm.touch.yaml
bottle log crm.touch who=ada channel=email note=pricing
bottle last crm.touch --where who=ada
bottle ls crm.touch --from 2026-08-01 --to 2026-08-31
```
