# The Kshana open test, protocol version 1

Written down on 21 October 2026, before any contributed record was read. The app shows
this exact text and its SHA-256 fingerprint on "The open test" page. Any change, even
one letter, changes the fingerprint; a changed protocol gets a new version number, the
old text stays published beside it, and records are analysed under the version they
were contributed with.

## 1. What is being tested

Hypothesis: firm dated predictions that Kshana seals before their window opens come
true more often than chance alone would make them come true. This is tested over all
firm predictions pooled (the primary test), and separately for each tier of firmness
and each area of life (secondary).

Null hypothesis: each prediction comes true with exactly its own chance probability
(defined in section 3), independently of the others.

## 2. The unit

One unit is one sealed claim that meets every one of these:

- It was sealed by the app (fingerprinted with SHA-256) before its window opened.
- It is firm: its tier is "strong" (the app gave it 0.75 or more) or "likely" (0.60 to
  under 0.75). Fainter readings are never sealed as firm and are not units.
- Its window has closed and the person answered it Yes, Close or No. "Not sure" answers
  are never units.
- It was never used to fit or correct the birth time.

A Yes is a hit. A No is a miss. In the primary test a Close is counted as a miss (the
strict reading); every result is also reported with Close counted as a hit (the
lenient reading).

## 3. The chance of each claim

Each claim carries its own chance: the probability it would be answered Yes if the
reading had no skill at all. Where the claim carries a measured chance (the share of
that person's comparable windows of the same length in which the same thing would have
been said, its "coverage"), that number is used. Where it does not, the chance is 1/2,
a coin, and the report says how many claims used the coin. Results are reported both
for all claims and for the claims with a measured chance only.

## 4. The primary metric

Hits K over N units are compared with the Poisson-binomial distribution of N
independent trials with the claims' own chances p1 ... pN: the exact probability that
chance alone gives K or more hits, P[X >= K]. This is a one-sided p-value. The primary
test is the pooled strict test over all units. Above 20,000 units the exact tail is
replaced by its normal approximation with continuity correction.

The overall false-alarm rate is 1 in 100 (alpha = 0.01). Because the data are looked at
more than once (section 6), that 0.01 is spent across the looks: 0.005 at the first
look, 0.0025 at the second, halving at each look after, so that across every look the
chance of wrongly announcing "better than chance" stays below 1 in 100. A look is
called "better than chance" only when its p-value is below that look's threshold and
the hit rate is above the mean chance.

## 5. Secondary metrics

- Brier skill score of the probability the app gave against the base rate:
  1 - Brier / (b (1 - b)), with b the share of hits. Above 0 means the app's stated
  probabilities did better than always saying the base rate.
- Hit rate with its Wilson 95% interval for each area of life, each tier (strong,
  likely) and each direction (good stretch, hard stretch), each with its mean chance
  and its own one-sided Poisson-binomial p-value, Holm-corrected across the areas and
  tiers. Groups with fewer than 20 units are listed but not tested.
- The same pooled test under the lenient reading, and over claims with a measured
  chance only.
- How long people took to answer after the window closed.

No other metric will be presented as a result of this test.

## 6. When it is analysed (the stopping rule)

The first analysis happens when 1,000 units have been pooled across everyone who
contributed. A further analysis happens at each additional 1,000 units (2,000,
3,000, ...). Nothing is announced between looks. The test never stops early because a
result looks good or bad.

## 7. Exclusions

These are removed before any analysis, and the report counts each kind:

- Duplicates: the same claim from the same install counted once (the first copy kept).
- Claims whose fingerprint no longer matches their words.
- Claims answered before their window closed, or with no answer.
- Claims used to fit the birth time.
- Claims of a tier other than strong or likely.
- Records made under a different protocol version (analysed under their own version).
- Lines that cannot be read, or that name a part of life the test does not know.

Nothing is excluded for any other reason, and never because of its outcome.

## 8. What a contributed record holds

Only what the test needs: the area of life, the tier, good or hard, the window's length
in days, the month the window opened (never the day), the claim's chance and how it was
measured, the probability the app gave, the answer, how many days after the window
closed it was answered, the app's engine version, this protocol's fingerprint, a
random code made on the phone for the install, and a code for each claim made from the
claim's fingerprint and that random code. No name, birth date, birth time, place, words
of the prediction, or anything the person wrote. A record leaves the phone only when
the person taps Share, and goes only where they send it.

## 9. Limits, said in advance

- People choose whether to contribute; those whose predictions came true may be more
  willing. The report says this plainly.
- Answers are the person's own judgement, not checked by anyone else.
- Records cannot be proved to come from real, separate people. Obvious copies are
  removed; nothing more can be promised.
- Claims of one person are not fully independent of each other.

## 10. The promise

Every analysis is published, whatever the result: better than chance, no better, or
worse. The full report, the counts of every exclusion and the evaluator's source code
are published together. Where an area of life does no better than chance, the app
stops giving firm dates for it.
