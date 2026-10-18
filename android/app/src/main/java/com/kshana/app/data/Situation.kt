package com.kshana.app.data

import android.content.Context
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import org.json.JSONArray
import org.json.JSONObject

/**
 * Something on a person's mind, in their words, mapped to what the engine answers: the
 * part of life, the matter, and the kind of question. The list covers what people most
 * often take to an astrologer (marriage and career first, then money, health, home,
 * travel, family, children, and the fear that something is wrong in the chart).
 */
data class SituationKind(
    val key: String,
    /** How the person would say it. */
    val says: String,
    /** Group it sits in on the picker. */
    val group: String,
    val life: String,
    /** A matter from the engine's list, or null for the life as a whole. */
    val topic: String?,
    /** The kind of question: will_it_happen, when, good_time_now, should_i, will_it_return, will_it_recover. */
    val question: String,
)

/** A situation the person chose, with when it began and their own note. */
data class MySituation(val key: String, val since: String?, val note: String)

object Situations {
    val ALL: List<SituationKind> = listOf(
        // Marriage and love
        SituationKind("marry_when", "When will I get married?", "Marriage and love", "Love", "marriage", "when"),
        SituationKind("reunion", "My partner left. Will we get back together?", "Marriage and love", "Love", "reunion", "will_it_return"),
        SituationKind("marriage_trouble", "Fights in my marriage. Will it settle?", "Marriage and love", "Love", "separation", "will_it_recover"),
        SituationKind("find_love", "Will I find the right person?", "Marriage and love", "Love", "new_love", "will_it_happen"),
        SituationKind("in_laws", "Trouble with my in-laws", "Marriage and love", "Love", "in_laws", "will_it_recover"),
        // Work
        SituationKind("job_when", "When will I get a job?", "Work", "Career", "new_job", "when"),
        SituationKind("promotion", "Will I get a promotion or a raise?", "Work", "Career", "promotion", "will_it_happen"),
        SituationKind("switch_job", "Should I change my job?", "Work", "Career", "new_job", "should_i"),
        SituationKind("own_business", "Should I start my own business?", "Work", "Career", "own_business", "should_i"),
        SituationKind("business_trouble", "My business is going badly. Will it recover?", "Work", "Career", "own_business", "will_it_recover"),
        SituationKind("govt_job", "Will I get a government job?", "Work", "Career", "government_post", "will_it_happen"),
        SituationKind("court_case", "A court case or dispute. Will I win?", "Work", "Career", "court_case", "will_it_happen"),
        // Money
        SituationKind("debt", "When will my debts clear?", "Money", "Money", "debt_cleared", "when"),
        SituationKind("money_stuck", "My money is stuck or lost. Will it come back?", "Money", "Money", null, "will_it_return"),
        SituationKind("loan", "Will my loan be approved?", "Money", "Money", "loan", "will_it_happen"),
        SituationKind("invest", "Is this a good time to invest?", "Money", "Money", "investment", "good_time_now"),
        // Abroad
        SituationKind("abroad_job", "Will I get a job abroad?", "Going abroad", "Career", "foreign_job", "will_it_happen"),
        SituationKind("visa", "Will my visa come through?", "Going abroad", "Travel", "visa", "will_it_happen"),
        SituationKind("settle_abroad", "When can I settle abroad?", "Going abroad", "Travel", "settle_abroad", "when"),
        SituationKind("return_home", "When can I come back home?", "Going abroad", "Travel", "return_home", "when"),
        // Health and mind
        SituationKind("recover", "I am unwell. When will I recover?", "Health and mind", "Health", "recovery", "will_it_recover"),
        SituationKind("surgery", "I have a surgery coming. Is the time right?", "Health and mind", "Health", "surgery", "good_time_now"),
        SituationKind("peace", "I have no peace of mind. When will it lift?", "Health and mind", "Health", "peace_of_mind", "when"),
        SituationKind("mother_health", "My mother's health worries me", "Health and mind", "Health", "mother_health", "will_it_recover"),
        SituationKind("father_health", "My father's health worries me", "Health and mind", "Health", "father_health", "will_it_recover"),
        // Home and family
        SituationKind("home_peace", "There is no peace at home", "Home and family", "Family", "home_peace", "will_it_recover"),
        SituationKind("sibling", "Trouble with a brother or sister", "Home and family", "Family", "sibling", "will_it_recover"),
        SituationKind("buy_house", "When can I buy a house or land?", "Home and family", "Home", "buy_house", "when"),
        SituationKind("build_house", "Is this the time to build a house?", "Home and family", "Home", "build_house", "good_time_now"),
        SituationKind("sell_house", "Will my house or land sell?", "Home and family", "Home", "sell_house", "will_it_happen"),
        // Studies
        SituationKind("exam", "Will I pass my exam?", "Studies", "Study", "competitive_exam", "will_it_happen"),
        SituationKind("admission", "Will I get the admission I want?", "Studies", "Study", "admission", "will_it_happen"),
        SituationKind("study_abroad", "Can I study abroad?", "Studies", "Study", "study_abroad", "will_it_happen"),
        // Children
        SituationKind("child_want", "We want a child. When?", "Children", "Children", "child_birth", "when"),
        SituationKind("child_studies", "My child's studies worry me", "Children", "Children", "child_studies", "will_it_recover"),
        SituationKind("child_health", "My child's health worries me", "Children", "Children", "child_health", "will_it_recover"),
        SituationKind("child_marriage", "When will my son or daughter marry?", "Children", "Children", "child_marriage", "when"),
    )

    /** A special worry with its own answer: an astrologer said something is wrong in my chart. */
    const val CHART_FEAR = "chart_fear"

    private const val KEY = "situations.json"
    var mine by mutableStateOf<List<MySituation>>(emptyList())
        private set

    fun kind(key: String): SituationKind? = ALL.firstOrNull { it.key == key }

    fun load(context: Context) {
        val raw = ProfileStore.prefs(context).getString(KEY, null)
        mine = runCatching {
            val a = JSONArray(raw ?: "[]")
            (0 until a.length()).map { i ->
                val o = a.getJSONObject(i)
                MySituation(o.getString("key"), o.optString("since").ifBlank { null }, o.optString("note"))
            }
        }.getOrDefault(emptyList())
    }

    private fun save(context: Context) {
        val a = JSONArray()
        for (s in mine) a.put(JSONObject().put("key", s.key).put("since", s.since ?: "").put("note", s.note))
        ProfileStore.prefs(context).edit().putString(KEY, a.toString()).apply()
    }

    fun toggle(context: Context, key: String) {
        mine = if (mine.any { it.key == key }) mine.filter { it.key != key } else mine + MySituation(key, null, "")
        save(context)
    }

    fun setSince(context: Context, key: String, since: String?) {
        mine = mine.map { if (it.key == key) it.copy(since = since) else it }
        save(context)
    }

    fun setNote(context: Context, key: String, note: String) {
        mine = mine.map { if (it.key == key) it.copy(note = note.take(200)) else it }
        save(context)
    }

    fun has(key: String) = mine.any { it.key == key }
}
