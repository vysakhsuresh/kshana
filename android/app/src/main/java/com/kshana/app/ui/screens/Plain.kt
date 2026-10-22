package com.kshana.app.ui.screens

/*
 * Plain words on the app side, mirroring kshana-rules' plain.rs. The engine sends some
 * fields as keys (a graha's name, a period's lord); the person never reads those keys.
 * These turn a key into what it means for the person's life.
 */

/** What an hour is for, in two or three words, for the hour tiles. */
internal fun hourShort(lord: String): String = when (lord) {
    "Sun" -> "Authority"
    "Moon" -> "Home & care"
    "Mars" -> "Hard work"
    "Mercury" -> "Talk & trade"
    "Jupiter" -> "Money & luck"
    "Venus" -> "Love & comfort"
    "Saturn" -> "Slow & steady"
    else -> "Uncertain"
}

/** What an hour is good for, as a phrase. */
internal fun hourNature(lord: String): String = when (lord) {
    "Sun" -> "a good hour to meet officials, seniors and anyone who can approve"
    "Moon" -> "a good hour for family, travel, food and caring work"
    "Mars" -> "a good hour for hard work, sport and repairs; keep away from arguments"
    "Mercury" -> "a good hour for calls, messages, study, writing and trade"
    "Jupiter" -> "a good hour for money, advice, learning, prayer and good beginnings"
    "Venus" -> "a good hour for love, shopping, beauty, art and making peace"
    "Saturn" -> "an hour for routine work, cleaning and patience; start nothing new"
    else -> "an uncertain hour; finish rather than begin"
}

/** What a stretch of life (a period) is about. */
internal fun chapterTheme(lord: String): String = when (lord) {
    "Sun" -> "standing, recognition and dealings with authority"
    "Moon" -> "home, family, feelings and peace of mind"
    "Mars" -> "effort, property, courage and competition"
    "Mercury" -> "learning, business, communication and paperwork"
    "Jupiter" -> "growth, money, children, teachers and faith"
    "Venus" -> "love, marriage, comfort and the good things of life"
    "Saturn" -> "work, duty, discipline and slow but lasting gains"
    "Rahu" -> "ambition, foreign links, technology and sudden turns"
    "Ketu" -> "letting go, inner growth and quiet endings"
    else -> "the ordinary run of life"
}

/** The first sentence of a passage. */
internal fun firstSentence(s: String): String {
    // A full stop or the Hindi danda ends a sentence.
    val i = listOf(s.indexOf(". "), s.indexOf("। ")).filter { it >= 0 }.minOrNull() ?: -1
    return if (i < 0) s.trim().let { if (it.endsWith(".") || it.endsWith("।")) it else "$it." } else s.substring(0, i + 1)
}

/** The kind of a remedy, as a label a person reads. */
internal fun kindWord(kind: String): String = when (kind) {
    "Anna" -> "FOOD"
    "Jeeva" -> "FEED"
    "Prakriti" -> "WATER & TREES"
    "Vastra" -> "WEAR"
    "Daana" -> "GIVE"
    "Karma", "Seva" -> "DO"
    "Discipline" -> "HABIT"
    "Vrata" -> "FAST"
    "Mantra" -> "CHANT"
    "Pooja" -> "PRAY"
    "Gem", "Ratna" -> "STONE"
    else -> kind.uppercase()
}

/** A match remedy's subject, as a label a person reads. */
internal fun doshaWord(dosha: String): String = when (dosha.lowercase()) {
    "nadi" -> "HEALTH"
    "bhakoot", "bhakut" -> "HARMONY"
    "gana" -> "TEMPER"
    "kuja", "mangal", "manglik" -> "QUARRELS"
    "yoni" -> "ATTRACTION"
    "graha maitri", "maitri" -> "FRIENDSHIP"
    else -> "THE MATCH"
}

/** "rising" → "getting better". */
internal fun trendWord(t: String): String = when (t) {
    "rising" -> "getting better"
    "falling" -> "getting harder"
    else -> "holding steady"
}

/** A long testing period's name, from its key. */
internal fun trialName(kind: String): String = when {
    kind.startsWith("sade") -> "The long seven-and-a-half-year test"
    kind.startsWith("ardh") -> "A test for home and peace of mind"
    kind.startsWith("ashtama") -> "A test for health and sudden changes"
    else -> "A testing time"
}

/** A chapter of life, as a short title. */
internal fun chapterShort(lord: String): String = when (lord) {
    "Sun" -> "Standing and recognition"
    "Moon" -> "Home and feelings"
    "Mars" -> "Effort and property"
    "Mercury" -> "Learning and business"
    "Jupiter" -> "Growth and money"
    "Venus" -> "Love and comfort"
    "Saturn" -> "Work and duty"
    "Rahu" -> "Ambition and sudden turns"
    "Ketu" -> "Letting go"
    else -> "A chapter"
}
