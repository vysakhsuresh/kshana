package com.kshana.app.ui.screens

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.DatePicker
import androidx.compose.material3.DatePickerDialog
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.rememberDatePickerState
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
import androidx.compose.ui.graphics.PathEffect
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
import uniffi.kshana_ffi.BriefingRecord
import uniffi.kshana_ffi.SampleRecord
import java.time.Instant
import java.time.LocalDate
import java.time.ZoneId
import java.time.ZoneOffset
import java.time.format.DateTimeFormatter

/**
 * Hindsight: pick a day that mattered and see what the sky over your chart said then:
 * the reading of that day, the periods and Saturn, and the slow line of a life for six
 * months either side with the day marked.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun HindsightScreen(profile: BirthProfile, place: Place, settings: EngineSettings, onBack: () -> Unit, onLife: (String) -> Unit) {
    val context = LocalContext.current
    var date by remember { mutableStateOf(HindsightRequest.date?.also { HindsightRequest.date = null } ?: LocalDate.now().minusYears(1)) }
    var pick by remember { mutableStateOf(false) }
    var brief by remember { mutableStateOf<BriefingRecord?>(null) }
    var slow by remember { mutableStateOf<List<SampleRecord>>(emptyList()) }
    var error by remember { mutableStateOf<String?>(null) }
    var life by remember { mutableStateOf("Love") }

    LaunchedEffect(date, profile, place, settings) {
        brief = null
        slow = emptyList()
        try {
            val engine = EngineProvider.get(context)
            val at = Instants.fromInstant(date.atTime(12, 0).atZone(ZoneId.systemDefault()).toInstant())
            val from = Instants.fromInstant(date.minusMonths(6).atTime(12, 0).atZone(ZoneId.systemDefault()).toInstant())
            val (b, s) = withContext(Dispatchers.Default) {
                Pair(
                    engine.briefing(profile.birthInstant(), profile.latitude, profile.longitude, place.latitude, place.longitude, settings.houseSystem, settings.ayanamsa, listOf("Love", "Career", "Money", "Health", "Family"), at, com.kshana.app.data.Lang.code),
                    engine.slowHorizon(profile.birthInstant(), profile.latitude, profile.longitude, place.latitude, place.longitude, settings.houseSystem, settings.ayanamsa, from, 13u),
                )
            }
            brief = b
            slow = s
            error = null
        } catch (e: Exception) {
            error = e.message ?: e.toString()
        }
    }

    if (pick) {
        val state = rememberDatePickerState(initialSelectedDateMillis = date.atStartOfDay(ZoneOffset.UTC).toInstant().toEpochMilli())
        DatePickerDialog(
            onDismissRequest = { pick = false },
            confirmButton = {
                TextButton(onClick = {
                    state.selectedDateMillis?.let { date = Instant.ofEpochMilli(it).atZone(ZoneOffset.UTC).toLocalDate() }
                    pick = false
                }) { Text("LOOK", color = KshanaColors.Gold) }
            },
            dismissButton = { TextButton(onClick = { pick = false }) { Text("CANCEL", color = KshanaColors.Muted) } },
        ) { DatePicker(state = state, title = null, headline = null, showModeToggle = false) }
    }

    Column(
        Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(horizontal = 24.dp).padding(top = 44.dp, bottom = 32.dp),
        verticalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween, verticalAlignment = Alignment.CenterVertically) {
            Text("LOOK BACK", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold)
            Text("BACK", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Muted, modifier = Modifier.clickable(onClick = onBack).padding(12.dp))
        }
        Text("How was that day for me?", style = MaterialTheme.typography.displayMedium, color = KshanaColors.Text)
        Text("Pick a day that mattered: a wedding, a fight, a job, a loss. See how that day stood for you, and the months around it.", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted)
        ValuePill(date.format(com.kshana.app.ui.screens.datePattern("EEEE d MMMM uuuu")), onClick = { pick = true }, modifier = Modifier.fillMaxWidth())
        ErrorLine(error)
        val b = brief
        if (b == null) {
            if (error == null) Working("reading that day…")
            return@Column
        }
        val m = b.moment
        Column(
            Modifier.fillMaxWidth().glass(18.dp).padding(18.dp),
            verticalArrangement = Arrangement.spacedBy(8.dp),
        ) {
            Text(m.dayTitle.uppercase(), style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold)
            Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(16.dp)) {
                ScoreRing(m.overall, size = 72.dp, stroke = 6.dp, label = bandWord(m.band))
                Column(Modifier.weight(1f)) {
                    Text(b.saturnWords + ".", style = MaterialTheme.typography.bodyMedium, color = if (b.saturnActive) KshanaColors.Ember else KshanaColors.TextSoft)
                }
            }
            for (p in b.paragraphs.drop(1)) Text(p, fontFamily = FontFamily.Serif, fontSize = 16.sp, lineHeight = 24.sp, color = KshanaColors.Text)
        }
        SectionLabel("Every part of your life that day")
        Column(verticalArrangement = Arrangement.spacedBy(6.dp)) {
            for (l in m.lives) {
                Row(Modifier.fillMaxWidth().clickable { life = l.life }.padding(vertical = 2.dp), verticalAlignment = Alignment.CenterVertically) {
                    Text("${lifeGlyph(l.life)} ${l.life}", style = MaterialTheme.typography.bodyMedium, color = if (life == l.life) KshanaColors.Gold else KshanaColors.Text, modifier = Modifier.width(100.dp))
                    Box(Modifier.weight(1f).height(6.dp).clip(RoundedCornerShape(3.dp)).background(KshanaColors.Line)) {
                        Box(Modifier.fillMaxWidth((l.score / 100.0).toFloat().coerceIn(0.02f, 1f)).height(6.dp).background(scoreColor(l.score)))
                    }
                    Spacer(Modifier.width(10.dp))
                    Text(l.score.toInt().toString(), fontFamily = FontFamily.Monospace, fontSize = 13.sp, color = scoreColor(l.score), modifier = Modifier.width(28.dp))
                }
            }
        }
        if (slow.size >= 2) {
            SectionLabel("The months around it · ${lifeName(life)}")
            val idx = LIFE_ORDER.indexOf(life).coerceAtLeast(0)
            val values = slow.map { it.scores.getOrNull(idx) ?: 50.0 }
            SlowAround(values, 6)
            Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween) {
                Text(slow.firstOrNull()?.let { "${dayMon(it.at)} ${localText(it.at, YEAR_ONLY)}" } ?: "", fontFamily = FontFamily.Monospace, fontSize = 10.sp, color = KshanaColors.Muted)
                Text(date.format(com.kshana.app.ui.screens.datePattern("d MMM uuuu")), fontFamily = FontFamily.Monospace, fontSize = 10.sp, color = KshanaColors.Gold)
                Text(slow.lastOrNull()?.let { "${dayMon(it.at)} ${localText(it.at, YEAR_ONLY)}" } ?: "", fontFamily = FontFamily.Monospace, fontSize = 10.sp, color = KshanaColors.Muted)
            }
            val here = values.getOrNull(6)
            val before = values.take(6).average()
            val after = values.drop(7).average()
            if (here != null) {
                Text(
                    "${lifeName(life)} stood at ${here.toInt()} that month, ${before.toInt()} in the six months before and ${after.toInt()} in the six months after.",
                    fontFamily = FontFamily.Serif, fontSize = 16.sp, lineHeight = 24.sp, color = KshanaColors.Text,
                )
            }
            QuietButton("OPEN $life TODAY", onClick = { onLife(life) }, modifier = Modifier.fillMaxWidth())
        }
    }
}

/** A slow line with the chosen month marked by a dashed line. */
@Composable
internal fun SlowAround(values: List<Double>, markIndex: Int) {
    if (values.size < 2) return
    val lo = (values.min() - 8.0).coerceAtLeast(0.0)
    val hi = (values.max() + 8.0).coerceAtMost(100.0)
    val span = (hi - lo).coerceAtLeast(30.0)
    val bottom = if (hi - lo < 30.0) ((lo + hi) / 2 - 15.0).coerceIn(0.0, 70.0) else lo
    Canvas(Modifier.fillMaxWidth().height(110.dp)) {
        val n = values.size
        val stepX = size.width / (n - 1)
        fun y(v: Double) = (size.height * (1 - ((v - bottom) / span).coerceIn(0.0, 1.0))).toFloat()
        for (g in listOf(32.0, 44.0, 56.0, 70.0)) {
            if (g > bottom && g < bottom + span) drawLine(KshanaColors.LineSoft, Offset(0f, y(g)), Offset(size.width, y(g)), 1f)
        }
        val mi = markIndex.coerceIn(0, n - 1)
        drawLine(KshanaColors.Gold.copy(alpha = 0.7f), Offset(mi * stepX, 0f), Offset(mi * stepX, size.height), 1.5f, pathEffect = PathEffect.dashPathEffect(floatArrayOf(6f, 6f)))
        val path = Path()
        values.forEachIndexed { i, v ->
            val p = Offset(i * stepX, y(v))
            if (i == 0) path.moveTo(p.x, p.y) else path.lineTo(p.x, p.y)
        }
        drawPath(path, KshanaColors.Sky, style = Stroke(2.5f, cap = StrokeCap.Round))
        drawCircle(KshanaColors.Gold, 5f, Offset(mi * stepX, y(values[mi])))
    }
}

/** A day another screen asked Look back to open on (the journal's "see that day"). */
internal object HindsightRequest {
    var date: LocalDate? = null
}
