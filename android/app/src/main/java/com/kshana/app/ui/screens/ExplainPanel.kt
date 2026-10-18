package com.kshana.app.ui.screens

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.kshana.app.data.BirthProfile
import com.kshana.app.data.EngineSettings
import com.kshana.app.data.Place
import com.kshana.app.engine.EngineProvider
import com.kshana.app.engine.Instants
import com.kshana.app.ui.theme.KshanaColors
import com.kshana.app.ui.theme.glass
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import uniffi.kshana_ffi.CauseRecord
import uniffi.kshana_ffi.ExplanationRecord

/**
 * Why a life stands where it stands: who causes it (each graha with its signed pull and
 * the exact instant it ends), what eases it (every coming change with its solved gain),
 * and the three-year slow line with its turning points.
 */
@Composable
fun ExplainPanel(profile: BirthProfile, place: Place, settings: EngineSettings, life: String, onRemedy: () -> Unit) {
    val context = LocalContext.current
    var open by remember { mutableStateOf(false) }
    var ex by remember { mutableStateOf<ExplanationRecord?>(null) }
    var error by remember { mutableStateOf<String?>(null) }
    var showAllRelief by remember { mutableStateOf(false) }

    LaunchedEffect(open, profile, place, settings, life) {
        if (!open || ex != null) return@LaunchedEffect
        try {
            val engine = EngineProvider.get(context)
            ex = withContext(Dispatchers.Default) {
                engine.explain(
                    profile.birthInstant(), profile.latitude, profile.longitude,
                    place.latitude, place.longitude,
                    settings.houseSystem, settings.ayanamsa, life, Instants.now(),
                )
            }
            error = null
        } catch (e: Exception) {
            error = e.message ?: e.toString()
        }
    }

    Column(
        Modifier.fillMaxWidth().glass(14.dp).padding(14.dp),
        verticalArrangement = Arrangement.spacedBy(8.dp),
    ) {
        Row(Modifier.fillMaxWidth().clickable { open = !open }, verticalAlignment = Alignment.CenterVertically) {
            Column(Modifier.weight(1f)) {
                Text("WHY NOW · WHAT WEIGHS · WHEN IT EASES", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold)
                Text(
                    if (open) "What presses on this part of your life, and the day each pressure lifts." else "Open it: what is pressing on this, when it lifts, and what helps.",
                    style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted,
                )
            }
            Text(if (open) "−" else "+", fontSize = 22.sp, color = KshanaColors.Gold)
        }
        if (!open) return@Column
        ErrorLine(error)
        val e = ex
        if (e == null) {
            if (error == null) Working("working out what comes next…")
            return@Column
        }
        Text(e.headline, fontFamily = FontFamily.Serif, fontSize = 18.sp, lineHeight = 26.sp, color = KshanaColors.Text)

        // Grahas with no measurable pull are left out, except Saturn and Jupiter, whose
        // position people always want to see.
        val keep: (CauseRecord) -> Boolean = { kotlin.math.abs(it.pull) >= 0.3 || it.planet == "Saturn" || it.planet == "Jupiter" || it.kind == "natal" }
        val causes = e.causes.filter(keep)
        val helpers = e.helpers.filter(keep)
        if (causes.isNotEmpty()) {
            SectionLabel("What weighs on it")
            for (c in causes) CauseRow(c, negative = true)
        }
        if (helpers.isNotEmpty()) {
            SectionLabel("What holds it up")
            for (c in helpers) CauseRow(c, negative = false)
        }

        SectionLabel("What changes, and by how much")
        val relief = if (showAllRelief) e.relief else e.relief.filter { it.lasting && (kotlin.math.abs(it.gain) >= 1.0 || it.planet == "Saturn" || it.planet == "Jupiter") }.take(8)
        if (relief.isEmpty()) Text("Nothing shifts within two years; the remedies are what you can do.", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted)
        for (r in relief) ReliefRow(r)
        if (e.relief.size > relief.size || showAllRelief) {
            Text(
                if (showAllRelief) "FEWER −" else "EVERY CHANGE, ${e.relief.size} +",
                style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold,
                modifier = Modifier.clickable { showAllRelief = !showAllRelief }.padding(vertical = 4.dp),
            )
        }
        Text("Each line is a date when something lifts or presses, and how much this part of your life gains or loses then.", style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted)

        SectionLabel("The next three years, month by month")
        SlowLine(e)
        Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween) {
            Text("now ${e.slowNow.toInt()}", fontFamily = FontFamily.Monospace, fontSize = 10.sp, color = KshanaColors.Muted)
            Text(e.slow.lastOrNull()?.let { "${dayMon(it.at)} ${localText(it.at, YEAR_ONLY)}" } ?: "", fontFamily = FontFamily.Monospace, fontSize = 10.sp, color = KshanaColors.Muted)
        }
        e.eases?.let { Bullet(it.words, KshanaColors.Leaf) }
        e.peak?.let { Bullet(it.words, KshanaColors.Gold) }
        e.toughens?.let { Bullet(it.words, KshanaColors.Ember) }
        if (e.eases == null && e.peak == null && e.toughens == null) {
            Text("The long view holds steady for three years: no big turn, only day-to-day ups and downs.", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted)
        }
        Text("This line is the long season, not the day-to-day weather.", style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted)
        if (e.score < 56) {
            Spacer(Modifier.height(2.dp))
            QuietButton("WHAT TO DO ABOUT IT", onClick = onRemedy, modifier = Modifier.fillMaxWidth())
        }
    }
}

@Composable
private fun CauseRow(c: CauseRecord, negative: Boolean) {
    val col = if (negative) KshanaColors.Ember else KshanaColors.Leaf
    HorizontalDivider(color = KshanaColors.LineSoft)
    Row(Modifier.fillMaxWidth().padding(vertical = 6.dp), verticalAlignment = Alignment.Top) {
        Text(if (negative) "▼" else "▲", fontSize = 12.sp, color = col, modifier = Modifier.width(26.dp).padding(top = 1.dp))
        Column(Modifier.weight(1f)) {
            Text(c.what.replaceFirstChar { it.uppercase() }, style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Text)
            if (c.untilWords.isNotBlank()) Text(c.untilWords.replaceFirstChar { it.uppercase() }, style = MaterialTheme.typography.bodySmall, color = KshanaColors.Sky)
            Text(kindName(c.kind), fontFamily = FontFamily.Monospace, fontSize = 10.sp, color = KshanaColors.Muted)
        }
        Text(signedPoints(c.pull), fontFamily = FontFamily.Monospace, fontSize = 15.sp, color = col, modifier = Modifier.padding(start = 8.dp))
    }
}

/** "+6", "−0.6": one decimal under two points so small pulls do not read as nothing. */
internal fun signedPoints(v: Double): String {
    val a = kotlin.math.abs(v)
    val body = if (a < 2.0) "%.1f".format(a) else a.toInt().toString()
    return (if (v < 0) "−" else "+") + body
}

private fun kindName(k: String): String = when (k) {
    "natal" -> "from birth"
    "dasha" -> "this stretch of your life"
    "gochara" -> "this season"
    "tara" -> "today's luck"
    "hora" -> "this hour"
    "kala" -> "the time of day"
    else -> k
}

@Composable
private fun SlowLine(e: ExplanationRecord) {
    val values = listOf(e.slowNow) + e.slow.map { it.scores.firstOrNull() ?: 50.0 }
    if (values.size < 2) return
    val lo = (values.min() - 8.0).coerceAtLeast(0.0)
    val hi = (values.max() + 8.0).coerceAtMost(100.0)
    val span = (hi - lo).coerceAtLeast(30.0)
    val bottom = if (hi - lo < 30.0) ((lo + hi) / 2 - 15.0).coerceIn(0.0, 70.0) else lo
    val easeIdx = e.eases?.let { t -> e.slow.indexOfFirst { it.at.taiSeconds == t.at.taiSeconds } + 1 }
    val peakIdx = e.peak?.let { t -> e.slow.indexOfFirst { it.at.taiSeconds == t.at.taiSeconds } + 1 }
    val toughIdx = e.toughens?.let { t -> e.slow.indexOfFirst { it.at.taiSeconds == t.at.taiSeconds } + 1 }
    Canvas(Modifier.fillMaxWidth().height(110.dp)) {
        val n = values.size
        val stepX = size.width / (n - 1)
        fun y(v: Double) = (size.height * (1 - ((v - bottom) / span).coerceIn(0.0, 1.0))).toFloat()
        for (g in listOf(32.0, 44.0, 56.0, 70.0)) {
            if (g > bottom && g < bottom + span) drawLine(KshanaColors.LineSoft, Offset(0f, y(g)), Offset(size.width, y(g)), 1f)
        }
        val path = Path()
        values.forEachIndexed { i, v ->
            val p = Offset(i * stepX, y(v))
            if (i == 0) path.moveTo(p.x, p.y) else path.lineTo(p.x, p.y)
        }
        drawPath(path, KshanaColors.Sky, style = Stroke(2.5f, cap = StrokeCap.Round))
        drawCircle(KshanaColors.Text, 4f, Offset(0f, y(values[0])))
        easeIdx?.takeIf { it in 1 until n }?.let { drawCircle(KshanaColors.Leaf, 5f, Offset(it * stepX, y(values[it]))) }
        peakIdx?.takeIf { it in 1 until n }?.let { drawCircle(KshanaColors.Gold, 5f, Offset(it * stepX, y(values[it]))) }
        toughIdx?.takeIf { it in 1 until n }?.let { drawCircle(KshanaColors.Ember, 5f, Offset(it * stepX, y(values[it]))) }
    }
}
