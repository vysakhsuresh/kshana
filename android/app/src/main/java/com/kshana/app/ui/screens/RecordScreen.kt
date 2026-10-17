package com.kshana.app.ui.screens

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
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.kshana.app.data.PredictionStore
import com.kshana.app.ui.theme.KshanaColors
import uniffi.kshana_ffi.calibrate
import java.time.Instant
import java.time.ZoneId

/** The proof feed: every named window that has passed, graded with one tap, and the record it makes. */
@Composable
fun RecordScreen(onBack: () -> Unit, onLife: (String) -> Unit) {
    val context = LocalContext.current
    var version by remember { mutableStateOf(0) }
    val pending = remember(version) { PredictionStore.pending(context).reversed() }
    val record = remember(version) { calibrate(PredictionStore.records(context)) }
    Column(
        Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(horizontal = 24.dp).padding(top = 44.dp, bottom = 32.dp),
        verticalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween, verticalAlignment = Alignment.CenterVertically) {
            Text("YOUR RECORD", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold)
            Text("BACK", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Muted, modifier = Modifier.clickable(onClick = onBack).padding(12.dp))
        }
        Text("Did it happen?", style = MaterialTheme.typography.displayMedium, color = KshanaColors.Text)
        Text("Every window the app named and that has passed. Say yes or no; the record is scored honestly, and after five answers for a life it is shown beside that life's score.", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted)

        Column(
            Modifier.fillMaxWidth().clip(RoundedCornerShape(16.dp)).background(KshanaColors.Surface).padding(16.dp),
            verticalArrangement = Arrangement.spacedBy(6.dp),
        ) {
            Text("SO FAR", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold)
            Text(record.words, fontFamily = FontFamily.Serif, fontSize = 18.sp, lineHeight = 26.sp, color = KshanaColors.Text)
            for (l in record.lives.filter { it.n > 0u }) {
                Row(Modifier.fillMaxWidth().clickable { onLife(l.life) }.padding(vertical = 2.dp), verticalAlignment = Alignment.CenterVertically) {
                    Text("${lifeGlyph(l.life)} ${l.life}", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.TextSoft, modifier = Modifier.width(100.dp))
                    Box(Modifier.weight(1f).height(5.dp).clip(RoundedCornerShape(2.dp)).background(KshanaColors.Line)) {
                        Box(Modifier.fillMaxWidth((l.rate / 100.0).toFloat().coerceIn(0.02f, 1f)).height(5.dp).background(scoreColor(l.rate)))
                    }
                    Spacer(Modifier.width(10.dp))
                    Text("${l.hits}/${l.n}", fontFamily = FontFamily.Monospace, fontSize = 12.sp, color = KshanaColors.Sky)
                    if (l.offset != 0.0) Text("  ${signedPoints(l.offset)}", fontFamily = FontFamily.Monospace, fontSize = 12.sp, color = KshanaColors.Gold)
                }
            }
        }

        SectionLabel("To grade · ${pending.size}")
        if (pending.isEmpty()) Text("Nothing waiting. Windows are remembered from the life pages and at each sunrise; they appear here once they have passed.", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted)
        for (p in pending) {
            HorizontalDivider(color = KshanaColors.LineSoft)
            val s = Instant.ofEpochMilli(p.start).atZone(ZoneId.systemDefault())
            val e = Instant.ofEpochMilli(p.end - 1).atZone(ZoneId.systemDefault())
            Row(Modifier.fillMaxWidth().padding(vertical = 6.dp), verticalAlignment = Alignment.CenterVertically) {
                Column(Modifier.weight(1f)) {
                    Text("${lifeGlyph(p.life)} ${p.life} · read ${p.score.toInt()}", style = MaterialTheme.typography.titleMedium, color = KshanaColors.Text)
                    Text(if (s.toLocalDate() == e.toLocalDate()) "${s.format(WEEKDAY_DAY)} · ${s.format(TIME_HM)} – ${e.format(TIME_HM)}" else "${s.format(WEEKDAY_DAY)} – ${e.format(WEEKDAY_DAY)}", fontFamily = FontFamily.Monospace, fontSize = 11.sp, color = KshanaColors.Sky)
                }
                Text("YES", style = MaterialTheme.typography.labelMedium, color = KshanaColors.Leaf, modifier = Modifier.clickable { PredictionStore.answer(context, p.life, p.start, true); version++ }.padding(10.dp))
                Text("NO", style = MaterialTheme.typography.labelMedium, color = KshanaColors.Ember, modifier = Modifier.clickable { PredictionStore.answer(context, p.life, p.start, false); version++ }.padding(10.dp))
            }
        }
    }
}
