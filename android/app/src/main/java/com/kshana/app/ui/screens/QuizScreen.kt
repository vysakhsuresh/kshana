package com.kshana.app.ui.screens

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontStyle
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.kshana.app.data.BirthProfile
import com.kshana.app.data.EngineSettings
import com.kshana.app.engine.EngineProvider
import com.kshana.app.engine.Instants
import com.kshana.app.ui.sky.KshanaFonts
import com.kshana.app.ui.theme.KshanaColors
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import uniffi.kshana_ffi.QuizQuestionRecord
import uniffi.kshana_ffi.QuizResultRecord
import uniffi.kshana_ffi.gradeQuiz
import uniffi.kshana_ffi.verifyQuizQuestion
import java.time.ZoneId

/** The eight life areas, by their engine names. */
private val QUIZ_AREAS = listOf("Career", "Marriage", "Children", "Wealth", "Health", "Property", "Education", "Foreign")

/** Questions asked, decoy windows beside the chart's own, and the age the lived span begins at. */
private val QUESTIONS: UInt = 8u
private val DECOYS: UInt = 3u
private const val LIVED_FROM_AGE = 12L

/** Answered questions below which the hit rate is shown as a count and a fraction, not a percentage. */
private const val MIN_FOR_PERCENT = 10

/** The answer "not sure", which the engine grades as skipped. */
private const val NOT_SURE = -1

/** One asked set: the committed questions and whether each commitment still matches its windows and key. */
private data class Asked(val area: String, val questions: List<QuizQuestionRecord>, val verified: List<Boolean>)

/**
 * Asked-first retrodiction: the chart commits to dated questions before you tell it
 * anything, each window beside decoys, and you say where the thing happened. The
 * grade is the hit rate next to chance, never alone. Nothing is stored.
 */
@Composable
fun QuizScreen(profile: BirthProfile, settings: EngineSettings, onBack: () -> Unit) {
    val context = LocalContext.current
    val scope = rememberCoroutineScope()
    val zone = remember { ZoneId.systemDefault() }

    var area by remember { mutableStateOf(QUIZ_AREAS[0]) }
    var asked by remember { mutableStateOf<Asked?>(null) }
    var asking by remember { mutableStateOf(false) }
    var error by remember { mutableStateOf<String?>(null) }
    var answers by remember { mutableStateOf<Map<Int, Int>>(emptyMap()) }
    var result by remember { mutableStateOf<QuizResultRecord?>(null) }

    fun ask() {
        if (asking) return
        asking = true
        error = null
        asked = null
        answers = emptyMap()
        result = null
        val chosen = area
        scope.launch {
            try {
                val engine = EngineProvider.get(context)
                val birth = profile.birthInstant()
                val from = Instants.fromInstant(profile.birthLocal().plusYears(LIVED_FROM_AGE).toInstant())
                val found = withContext(Dispatchers.Default) {
                    val qs = engine.retrodictionQuiz(birth, profile.latitude, profile.longitude, settings.ayanamsa, chosen, from, Instants.now(), QUESTIONS, DECOYS)
                    Asked(chosen, qs, qs.map { runCatching { verifyQuizQuestion(it) }.getOrDefault(false) })
                }
                asked = found
            } catch (e: Exception) {
                error = e.message ?: e.toString()
            }
            asking = false
        }
    }

    fun grade() {
        val set = asked ?: return
        val list = set.questions.indices.map { (answers[it] ?: NOT_SURE).toLong() }
        result = runCatching { gradeQuiz(set.questions, list) }.onFailure { error = it.message ?: it.toString() }.getOrNull()
    }

    Column(
        Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(horizontal = 24.dp).padding(top = 48.dp, bottom = 32.dp),
        verticalArrangement = Arrangement.spacedBy(10.dp),
    ) {
        Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween, verticalAlignment = Alignment.CenterVertically) {
            Text("ASKED FIRST", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold)
            Text("BACK", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Muted, modifier = Modifier.clickable(onClick = onBack).padding(12.dp))
        }
        Text("The chart asks, you answer", style = MaterialTheme.typography.displayMedium, color = KshanaColors.Text)
        Text(
            "Before you tell it anything, the chart picks the stretches of your life it would call strongest for an area and hides each among ${DECOYS.toInt()} decoys of similar length. You say in which one the thing happened. Nothing is stored.",
            style = MaterialTheme.typography.bodyMedium, color = KshanaColors.TextSoft,
        )

        SectionLabel("Area")
        Row(Modifier.fillMaxWidth().horizontalScroll(rememberScrollState()), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            for (a in QUIZ_AREAS) Chip(a.uppercase(), selected = area == a, onClick = { area = a })
        }
        Text(
            "From age ${LIVED_FROM_AGE} to today, in ${zone.id}.",
            fontFamily = KshanaFonts.mono, fontSize = 11.sp, color = KshanaColors.Sky,
        )
        GoldButton(if (asking) "COMMITTING…" else "ASK", enabled = !asking, onClick = { ask() }, modifier = Modifier.fillMaxWidth())
        if (asking) {
            Text("Reading every sub-period of the span and committing ${QUESTIONS.toInt()} questions…", style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted)
        }
        ErrorLine(error)

        val set = asked
        if (set != null) {
            if (set.questions.isEmpty()) {
                Text("The span holds too few sub-periods to ask about ${set.area} with ${DECOYS.toInt()} decoys each.", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted)
            } else {
                SectionLabel("Questions")
                Text(
                    "Each question carries a commitment: the SHA-256 of its windows and its answer key, computed by the engine before the question was shown to you. The chart's own window was fixed then and is revealed only when you grade.",
                    style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted,
                )
                for ((i, q) in set.questions.withIndex()) {
                    QuestionCard(
                        index = i,
                        total = set.questions.size,
                        q = q,
                        verified = set.verified.getOrElse(i) { false },
                        zone = zone,
                        answer = answers[i],
                        graded = result != null,
                        onAnswer = { answers = answers + (i to it) },
                    )
                }
                val answered = set.questions.indices.count { answers.containsKey(it) }
                if (result == null) {
                    Text(
                        if (answered < set.questions.size) "$answered of ${set.questions.size} answered; answer every question, or say you are not sure, to grade."
                        else "Every question answered.",
                        style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted,
                    )
                    GoldButton("GRADE", enabled = answered == set.questions.size, onClick = { grade() }, modifier = Modifier.fillMaxWidth())
                }
            }
        }

        val r = result
        if (r != null) {
            SectionLabel("Grade")
            Row(verticalAlignment = Alignment.Bottom, horizontalArrangement = Arrangement.spacedBy(10.dp)) {
                Text("${r.correct}", fontFamily = KshanaFonts.mono, fontSize = 44.sp, color = KshanaColors.Sky)
                Text("of ${r.total}", fontFamily = KshanaFonts.mono, fontSize = 20.sp, color = KshanaColors.Muted, modifier = Modifier.padding(bottom = 6.dp))
            }
            Text(gradeLine(r), fontFamily = KshanaFonts.mono, fontSize = 12.sp, color = KshanaColors.Sky)
            Text(r.words, fontFamily = KshanaFonts.serif, fontSize = 17.sp, lineHeight = 24.sp, color = KshanaColors.Text)
            Text(
                "Chance is the forced-choice rate, one window in ${DECOYS.toInt() + 1}; p is the exact binomial tail of scoring at least this well by guessing; the interval is Wilson's at 95%.",
                style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted,
            )
            QuietButton("ASK AGAIN", onClick = { ask() }, modifier = Modifier.fillMaxWidth())
        }

        Spacer(Modifier.height(12.dp))
        Text(
            "Eight questions cannot settle much either way. The dasha part of every window's strength is exact; the transit part is sampled once per pratyantar cell.",
            fontFamily = KshanaFonts.serif, fontStyle = FontStyle.Italic, fontSize = 14.sp, lineHeight = 20.sp, color = KshanaColors.Muted,
        )
    }
}

/** "3 of 8 · chance 1 in 4 · p = 0.321 · hit rate 0.14–0.69": counts and fractions below ten answers, percentages from ten. */
private fun gradeLine(r: QuizResultRecord): String {
    val total = r.total.toInt()
    val chance = if (r.chanceRate > 0.0) "chance 1 in ${"%.0f".format(1.0 / r.chanceRate)}" else "chance 0"
    val interval = if (total >= MIN_FOR_PERCENT) {
        "hit rate %.0f%% (95%% %.0f–%.0f%%)".format(100.0 * r.correct.toInt() / total, 100.0 * r.hitLo, 100.0 * r.hitHi)
    } else {
        "hit rate 95%% %.2f–%.2f".format(r.hitLo, r.hitHi)
    }
    val skipped = if (r.skipped > 0u) " · ${r.skipped} skipped" else ""
    val p = "%.3f".format(r.binomialPvalue)
    return "${r.correct} of $total · $chance · p = $p · $interval$skipped"
}

/** One committed question: the prompt, its windows to tap, not sure, and the commitment. */
@Composable
private fun QuestionCard(
    index: Int,
    total: Int,
    q: QuizQuestionRecord,
    verified: Boolean,
    zone: ZoneId,
    answer: Int?,
    graded: Boolean,
    onAnswer: (Int) -> Unit,
) {
    val shape = RoundedCornerShape(8.dp)
    Column(
        Modifier.fillMaxWidth().clip(shape).background(KshanaColors.Surface).border(1.dp, KshanaColors.Line, shape).padding(16.dp),
        verticalArrangement = Arrangement.spacedBy(8.dp),
    ) {
        Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
            Text("QUESTION ${index + 1} OF $total", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold, modifier = Modifier.weight(1f))
            if (!verified) Text("ALTERED", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Sky)
        }
        Text(q.prompt, fontFamily = KshanaFonts.serif, fontSize = 16.sp, lineHeight = 22.sp, color = KshanaColors.Text)
        val n = minOf(q.starts.size, q.ends.size)
        for (w in 0 until n) {
            val chosen = answer == w
            val isChart = graded && w == q.predictedIndex.toInt()
            val border = when {
                isChart -> KshanaColors.Gold
                chosen -> KshanaColors.Sky
                else -> KshanaColors.Line
            }
            Row(
                Modifier.fillMaxWidth().heightIn(min = 44.dp).clip(RoundedCornerShape(6.dp)).border(1.dp, border, RoundedCornerShape(6.dp))
                    .clickable(enabled = !graded) { onAnswer(w) }.padding(horizontal = 12.dp, vertical = 8.dp),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Text(
                    "${w + 1} · ${localText(q.starts[w], DATE_ONLY, zone)} – ${localText(q.ends[w], DATE_ONLY, zone)}",
                    fontFamily = KshanaFonts.mono, fontSize = 12.sp,
                    color = if (chosen || isChart) KshanaColors.Text else KshanaColors.TextSoft,
                    modifier = Modifier.weight(1f),
                )
                if (chosen) Text("YOURS", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Sky, modifier = Modifier.padding(start = 8.dp))
                if (isChart) Text("CHART", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold, modifier = Modifier.padding(start = 8.dp))
            }
        }
        Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
            Chip("NOT SURE", selected = answer == NOT_SURE, onClick = { if (!graded) onAnswer(NOT_SURE) })
            Spacer(Modifier.weight(1f))
            if (graded) {
                Text(
                    when (answer) {
                        null, NOT_SURE -> "skipped"
                        q.predictedIndex.toInt() -> "the chart's window"
                        else -> "not the chart's window"
                    },
                    style = MaterialTheme.typography.bodySmall,
                    color = if (answer == q.predictedIndex.toInt()) KshanaColors.Gold else KshanaColors.Muted,
                )
            }
        }
        Text("commitment ${q.commitment.take(16)}… · sha-256, before asking", fontFamily = KshanaFonts.mono, fontSize = 9.sp, color = KshanaColors.Muted)
    }
}
