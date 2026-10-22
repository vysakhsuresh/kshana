package com.kshana.app.ui.screens

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.IntrinsicSize
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.verticalScroll
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
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.kshana.app.data.BirthProfile
import com.kshana.app.data.EngineSettings
import com.kshana.app.data.Focus
import com.kshana.app.data.Lang
import com.kshana.app.data.Place
import com.kshana.app.engine.EngineProvider
import com.kshana.app.engine.Instants
import com.kshana.app.ui.theme.KshanaColors
import com.kshana.app.ui.theme.glass
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import uniffi.kshana_ffi.FutureEventRecord
import java.time.format.DateTimeFormatter

internal object FutureCache {
    val lists = HashMap<String, List<FutureEventRecord>>()
}

private val RANGES = listOf(12 to "1 YEAR", 36 to "3 YEARS", 60 to "5 YEARS")
private val FILTERS = listOf("all" to "EVERYTHING", "opening" to "OPENINGS", "pressure" to "HARD STRETCHES", "turn" to "TURNS OF LIFE")

private fun kindColour(kind: String): Color = when (kind) {
    "opening" -> KshanaColors.Leaf
    "pressure" -> KshanaColors.Ember
    "chapter" -> KshanaColors.Gold
    "test_ends", "test_pause" -> KshanaColors.Sky
    else -> KshanaColors.Ember
}

/**
 * The future, by itself: what is coming, in order, for the parts of life the person
 * chose. Openings, hard stretches, the turns of the long chapters of life and the long
 * testing periods, each with its months, how strong it is and what to do.
 */
@Composable
fun FutureScreen(profile: BirthProfile, place: Place, settings: EngineSettings, onRemedy: (String?) -> Unit, onSettings: () -> Unit, onWall: () -> Unit = {}, onBook: () -> Unit = {}) {
    val context = LocalContext.current
    var months by remember { mutableStateOf(36) }
    var filter by remember { mutableStateOf("all") }
    var list by remember { mutableStateOf<List<FutureEventRecord>?>(null) }
    var error by remember { mutableStateOf<String?>(null) }
    val key = "${profile.birthInstant().taiSeconds}|${place.name}|$months|${Lang.code}|${Focus.lives}|${java.time.LocalDate.now()}"
    LaunchedEffect(key) {
        list = FutureCache.lists[key]
        if (list != null) return@LaunchedEffect
        try {
            val engine = EngineProvider.get(context)
            val l = withContext(Dispatchers.Default) {
                engine.futurePersonal(profile.birthInstant(), profile.latitude, profile.longitude, place.latitude, place.longitude, settings.houseSystem, settings.ayanamsa, Instants.now(), months.toUInt(), Lang.code, Focus.lives, com.kshana.app.data.PersonalWeights.input(context, profile.name))
            }
            FutureCache.lists[key] = l
            list = l
            error = null
        } catch (e: Exception) {
            error = e.message ?: e.toString()
        }
    }

    Column(
        Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(horizontal = 24.dp).padding(top = 44.dp, bottom = 32.dp),
        verticalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        Text("THE FUTURE", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold)
        Text("What is coming", style = MaterialTheme.typography.displayMedium, color = KshanaColors.Text)
        Text(
            "In order, with the months, how strong each one is, and what to do. Only for the parts of life you chose.",
            style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted,
        )
        Row(verticalAlignment = Alignment.CenterVertically) {
            Text(
                Focus.lives.joinToString(" · ") { lifeName(it) },
                style = MaterialTheme.typography.bodySmall, color = KshanaColors.Sky, modifier = Modifier.weight(1f),
            )
            Text("CHANGE ›", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold, modifier = Modifier.clickable(onClick = onSettings).padding(8.dp))
        }
        Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            for ((m, label) in RANGES) Chip(label, selected = months == m, onClick = { months = m })
        }
        Row(Modifier.fillMaxWidth().horizontalScroll(rememberScrollState()), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            for ((f, label) in FILTERS) OptionBox(label, selected = filter == f, onClick = { filter = f })
        }
        ErrorLine(error)
        val all = list
        if (all == null) {
            if (error == null) Working("looking ahead, month by month…")
            return@Column
        }
        val shown = all.filter {
            when (filter) {
                "opening" -> it.kind == "opening"
                "pressure" -> it.kind == "pressure" || it.kind == "test_begins" || it.kind == "test_returns"
                "turn" -> it.kind != "opening" && it.kind != "pressure"
                else -> true
            }
        }
        val openings = all.count { it.kind == "opening" }
        val hard = all.count { it.kind == "pressure" }
        Text(
            "$openings openings and $hard stretches to tend ahead in ${months / 12} year${if (months > 12) "s" else ""}.",
            fontFamily = FontFamily.Serif, fontSize = 18.sp, color = KshanaColors.Text,
        )
        if (shown.isEmpty()) Text("Nothing of this kind in this span.", style = MaterialTheme.typography.bodyLarge, color = KshanaColors.TextSoft)
        val mon = com.kshana.app.ui.screens.datePattern("MMM")
        val yr = DateTimeFormatter.ofPattern("uuuu")
        for (e in shown) FutureRow(e, mon, yr, onRemedy)
        Spacer(Modifier.height(4.dp))
        Text(
            "This is the weather of each season, not a fixed fate. A testing stretch eased by the right deeds is lighter; an opening not taken passes. Your choices count.",
            style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted,
        )
        // The pages that carry this on: dates counting down, and the whole life as chapters.
        Row(horizontalArrangement = Arrangement.spacedBy(18.dp)) {
            Text("COUNT DOWN TO WHAT MATTERS ›", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold, modifier = Modifier.clickable(onClick = onWall).padding(vertical = 6.dp))
            Text("THE BOOK OF YOUR LIFE ›", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold, modifier = Modifier.clickable(onClick = onBook).padding(vertical = 6.dp))
        }
    }
}

@Composable
private fun FutureRow(e: FutureEventRecord, mon: DateTimeFormatter, yr: DateTimeFormatter, onRemedy: (String?) -> Unit) {
    val context = LocalContext.current
    val c = kindColour(e.kind)
    Row(Modifier.fillMaxWidth().height(IntrinsicSize.Min)) {
        // The date gutter and the thread of time.
        Column(Modifier.width(56.dp), horizontalAlignment = Alignment.End) {
            Text(Instants.toLocal(e.from).format(mon).uppercase(), fontFamily = FontFamily.Monospace, fontSize = 12.sp, color = KshanaColors.Sky)
            Text(Instants.toLocal(e.from).format(yr), fontFamily = FontFamily.Monospace, fontSize = 11.sp, color = KshanaColors.Muted)
        }
        Spacer(Modifier.width(10.dp))
        Box(Modifier.width(12.dp).fillMaxHeight(), contentAlignment = Alignment.TopCenter) {
            Box(Modifier.width(1.dp).fillMaxHeight().background(KshanaColors.Line))
            Box(Modifier.padding(top = 4.dp).size(10.dp).clip(CircleShape).background(c))
        }
        Spacer(Modifier.width(10.dp))
        Column(Modifier.weight(1f).padding(bottom = 14.dp).glass(14.dp).padding(14.dp), verticalArrangement = Arrangement.spacedBy(6.dp)) {
            Text(
                (e.life?.let { "${lifeGlyph(it)} ${lifeName(it).uppercase()}" } ?: "YOUR WHOLE LIFE") + " · " + when (e.kind) {
                    "opening" -> "AN OPENING"
                    "pressure" -> "A STRETCH TO TEND"
                    "chapter" -> "A NEW CHAPTER"
                    "test_begins" -> "A TESTING TIME BEGINS"
                    "test_returns" -> "THE TESTING TIME RETURNS"
                    "test_pause" -> "A BREATHING SPACE"
                    else -> "A TESTING TIME ENDS"
                },
                style = MaterialTheme.typography.labelSmall, color = c,
            )
            Text(e.headline, fontFamily = FontFamily.Serif, fontSize = 17.sp, lineHeight = 24.sp, color = KshanaColors.Text)
            Text(e.detail, style = MaterialTheme.typography.bodyMedium, color = KshanaColors.TextSoft)
            Row(Modifier.horizontalScroll(rememberScrollState()), horizontalArrangement = Arrangement.spacedBy(14.dp)) {
                // Every stretch that asks something of the person carries what to do about it;
                // a whole-life one (no life named) opens the general remedies.
                if (e.kind == "pressure" || e.kind == "test_begins" || e.kind == "test_returns") {
                    Text("WHAT TO DO ›", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold, modifier = Modifier.clickable { onRemedy(e.life) }.padding(vertical = 4.dp))
                }
                Text("REMIND ME ›", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold, modifier = Modifier.clickable {
                    val at = Instants.toLocal(e.from).toInstant().toEpochMilli()
                    com.kshana.app.system.Alerts.toggle(context, at, e.headline.take(60), e.detail)
                    android.widget.Toast.makeText(context, "You will be told when it begins.", android.widget.Toast.LENGTH_SHORT).show()
                }.padding(vertical = 4.dp))
                Text("CALENDAR ›", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold, modifier = Modifier.clickable {
                    addToCalendar(context, "Kshana · " + e.headline.take(60), Instants.toLocal(e.from).toInstant().toEpochMilli(), Instants.toLocal(e.to).toInstant().toEpochMilli(), e.detail)
                }.padding(vertical = 4.dp))
                Text("SHARE ›", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold, modifier = Modifier.clickable {
                    com.kshana.app.system.ShareCard.share(context, "What is coming", e.headline, e.detail)
                }.padding(vertical = 4.dp))
            }
        }
    }
}
