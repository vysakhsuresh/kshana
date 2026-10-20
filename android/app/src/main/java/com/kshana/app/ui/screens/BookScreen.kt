package com.kshana.app.ui.screens

import androidx.compose.foundation.background
import androidx.compose.foundation.border
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
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
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
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.kshana.app.data.BirthProfile
import com.kshana.app.data.EngineSettings
import com.kshana.app.engine.EngineProvider
import com.kshana.app.engine.Instants
import com.kshana.app.ui.theme.KshanaColors
import com.kshana.app.ui.theme.glass
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import com.kshana.app.data.Place
import uniffi.kshana_ffi.ChapterRecord
import uniffi.kshana_ffi.TimelineRecord
import uniffi.kshana_ffi.YearChapterRecord

/** The book depends only on the birth, so one reading per profile is kept for the process. */
internal object BookCache {
    var key: String? = null
    var record: TimelineRecord? = null
}

private val PAST_ALPHA = 0.55f

/**
 * The book of a life: chapters and pages (Vimshottari), Saturn's trials from birth,
 * Jupiter's passages, and the returns, with the present marked. Past, present, future.
 */
internal object YearCache {
    var key: String? = null
    var record: YearChapterRecord? = null
}

/** This year, birthday to birthday: the solar return's rising sign and lord, a tone per life. */
@Composable
private fun YearSection(profile: BirthProfile, place: Place, settings: EngineSettings, onLife: (String) -> Unit) {
    val context = LocalContext.current
    val key = "${profile.birthInstant().taiSeconds}|${place.name}|${settings.ayanamsa}|${settings.houseSystem}|${java.time.LocalDate.now()}"
    var y by remember { mutableStateOf(if (YearCache.key == key) YearCache.record else null) }
    var error by remember { mutableStateOf<String?>(null) }
    LaunchedEffect(key) {
        if (y != null) return@LaunchedEffect
        try {
            val engine = EngineProvider.get(context)
            val r = withContext(Dispatchers.Default) {
                engine.yearChapter(profile.birthInstant(), profile.latitude, profile.longitude, place.latitude, place.longitude, settings.houseSystem, settings.ayanamsa, Instants.now())
            }
            YearCache.key = key
            YearCache.record = r
            y = r
        } catch (e: Exception) {
            error = e.message ?: e.toString()
        }
    }
    SectionLabel("Your year · birthday to birthday")
    ErrorLine(error)
    val r = y
    if (r == null) {
        if (error == null) Working("reading your year…")
        return
    }
    Column(
        Modifier.fillMaxWidth().glass(14.dp).padding(14.dp),
        verticalArrangement = Arrangement.spacedBy(6.dp),
    ) {
        Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
            Column(Modifier.weight(1f)) {
                Text("A year of ${chapterShort(r.lord).lowercase()}", fontFamily = FontFamily.Serif, fontSize = 18.sp, color = KshanaColors.Text)
                Text("${localText(r.start, DATE_ONLY)} – ${localText(r.end, DATE_ONLY)}", fontFamily = FontFamily.Monospace, fontSize = 11.sp, color = KshanaColors.Sky)
            }
            Text(r.tone.toInt().toString(), fontFamily = FontFamily.Monospace, fontSize = 22.sp, color = scoreColor(r.tone))
        }
        Text(r.headline, fontFamily = FontFamily.Serif, fontSize = 16.sp, lineHeight = 24.sp, color = KshanaColors.Text)
        Text(r.theme, style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted)
        HorizontalDivider(color = KshanaColors.LineSoft)
        for ((i, name) in LIFE_ORDER.withIndex()) {
            val v = r.tones.getOrNull(i) ?: 50.0
            Row(Modifier.fillMaxWidth().clickable { onLife(name) }.padding(vertical = 2.dp), verticalAlignment = Alignment.CenterVertically) {
                Text("${lifeGlyph(name)} $name", style = MaterialTheme.typography.bodySmall, color = KshanaColors.TextSoft, modifier = Modifier.width(96.dp))
                Box(Modifier.weight(1f).height(5.dp).clip(RoundedCornerShape(2.dp)).background(KshanaColors.Line)) {
                    Box(Modifier.fillMaxWidth((v / 100.0).toFloat().coerceIn(0.02f, 1f)).height(5.dp).background(scoreColor(v)))
                }
                Spacer(Modifier.width(8.dp))
                Text(v.toInt().toString(), fontFamily = FontFamily.Monospace, fontSize = 11.sp, color = scoreColor(v), modifier = Modifier.width(24.dp))
            }
        }
    }
}

@Composable
fun BookScreen(profile: BirthProfile, place: Place, settings: EngineSettings, onBack: () -> Unit, onLife: (String) -> Unit = {}, onRemedy: () -> Unit = {}) {
    val context = LocalContext.current
    val key = "${profile.birthInstant().taiSeconds}|${profile.latitude}|${profile.longitude}|${settings.ayanamsa}|${settings.houseSystem}"
    var book by remember { mutableStateOf(if (BookCache.key == key) BookCache.record else null) }
    var error by remember { mutableStateOf<String?>(null) }
    var openChapter by remember { mutableStateOf<Int?>(null) }
    var showAllJupiter by remember { mutableStateOf(false) }

    LaunchedEffect(key) {
        if (book != null) return@LaunchedEffect
        try {
            val engine = EngineProvider.get(context)
            val t = withContext(Dispatchers.Default) {
                engine.timeline(profile.birthInstant(), profile.latitude, profile.longitude, settings.houseSystem, settings.ayanamsa, Instants.now())
            }
            BookCache.key = key
            BookCache.record = t
            book = t
            error = null
        } catch (e: Exception) {
            error = e.message ?: e.toString()
        }
    }

    Column(
        Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(horizontal = 24.dp).padding(top = 44.dp, bottom = 32.dp),
        verticalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween, verticalAlignment = Alignment.CenterVertically) {
            Text("THE BOOK", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold)
            Text("BACK", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Muted, modifier = Modifier.clickable(onClick = onBack).padding(12.dp))
        }
        Text("The book of your life", style = MaterialTheme.typography.displayMedium, color = KshanaColors.Text)
        ErrorLine(error)
        val t = book
        if (t == null) {
            if (error == null) Working("reading your whole life…")
            return@Column
        }
        Text(t.headline, fontFamily = FontFamily.Serif, fontSize = 19.sp, lineHeight = 27.sp, color = KshanaColors.Text)

        YearSection(profile, place, settings, onLife)

        SectionLabel("The testing stretches of your life")
        Text("Every long testing period from your birth to the years ahead, to the day.", style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted)
        for (s in t.saturn) {
            val past = s.end.seconds() < t.at.seconds()
            val alpha = if (past) PAST_ALPHA else 1f
            Column(
                Modifier.fillMaxWidth().clip(RoundedCornerShape(12.dp))
                    .background(if (s.current) KshanaColors.Ember.copy(alpha = 0.12f) else KshanaColors.Surface)
                    .then(if (s.current) Modifier.border(1.dp, KshanaColors.Ember, RoundedCornerShape(12.dp)) else Modifier)
                    .padding(12.dp),
                verticalArrangement = Arrangement.spacedBy(4.dp),
            ) {
                Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween) {
                    Text(trialName(s.kind), fontFamily = FontFamily.Serif, fontSize = 17.sp, color = (if (s.current) KshanaColors.Ember else KshanaColors.Text).copy(alpha = alpha))
                    Text("age ${s.age.toInt()}", fontFamily = FontFamily.Monospace, fontSize = 11.sp, color = KshanaColors.Muted.copy(alpha = alpha))
                }
                Text("${dayMon(s.start)} ${localText(s.start, YEAR_ONLY)} – ${dayMon(s.end)} ${localText(s.end, YEAR_ONLY)}", fontFamily = FontFamily.Monospace, fontSize = 12.sp, color = KshanaColors.Sky.copy(alpha = alpha))
                if (s.current) {
                    Text("You are in it now. It ends ${localText(s.end, DATE_TIME)}.", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Text)
                    Text("WHAT HELPS NOW ›", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold, modifier = Modifier.clickable(onClick = onRemedy).padding(vertical = 4.dp))
                    for (p in s.phases) {
                        Text("${localText(p.at, DATE_ONLY)} · ${p.words}", style = MaterialTheme.typography.bodySmall, color = if (p.at.seconds() <= t.at.seconds()) KshanaColors.Muted else KshanaColors.Sky)
                    }
                } else {
                    Text(if (past) "Passed." else "Ahead.", style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted.copy(alpha = alpha))
                }
            }
        }

        SectionLabel("The big chapters of your life")
        for ((i, c) in t.chapters.withIndex()) {
            ChapterRow(c, t, open = openChapter == i, onToggle = { openChapter = if (openChapter == i) null else i }, onLife = onLife)
        }

        SectionLabel("Inside the present chapter")
        for ((i, c) in t.pages.withIndex()) {
            ChapterRow(c, t, open = openChapter == 100 + i, onToggle = { openChapter = if (openChapter == 100 + i) null else 100 + i }, onLife = onLife)
        }

        SectionLabel("Your lucky years and lean years")
        Text("When fortune helps you and when it holds back, from three years ago to twelve years ahead.", style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted)
        val jup = if (showAllJupiter) t.jupiter else t.jupiter.filter { it.end.seconds() >= t.at.seconds() }.take(8)
        for (p in jup) {
            val past = p.end.seconds() < t.at.seconds()
            val alpha = if (past) PAST_ALPHA else 1f
            Row(Modifier.fillMaxWidth().padding(vertical = 3.dp), verticalAlignment = Alignment.Top) {
                Box(Modifier.padding(top = 6.dp).size(8.dp).clip(CircleShape).background((if (p.favourable) KshanaColors.Leaf else KshanaColors.Muted).copy(alpha = alpha)))
                Spacer(Modifier.width(10.dp))
                Column(Modifier.weight(1f)) {
                    Text("${dayMon(p.start)} ${localText(p.start, YEAR_ONLY)} – ${dayMon(p.end)} ${localText(p.end, YEAR_ONLY)}${if (p.current) "  · now" else ""}", fontFamily = FontFamily.Monospace, fontSize = 11.sp, color = (if (p.current) KshanaColors.Gold else KshanaColors.Sky).copy(alpha = alpha))
                    Text(p.words, style = MaterialTheme.typography.bodyMedium, color = KshanaColors.TextSoft.copy(alpha = alpha))
                }
            }
        }
        Text(
            if (showAllJupiter) "FEWER −" else "ALL ${t.jupiter.size} +",
            style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold,
            modifier = Modifier.clickable { showAllJupiter = !showAllJupiter }.padding(vertical = 4.dp),
        )

        SectionLabel("Turning points")
        Text("The years when an old cycle of your life comes round again.", style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted)
        for (r in t.returns) {
            val past = r.at.seconds() < t.at.seconds()
            val alpha = if (past) PAST_ALPHA else 1f
            Row(Modifier.fillMaxWidth().padding(vertical = 3.dp), verticalAlignment = Alignment.Top) {
                Text(localText(r.at, DATE_ONLY), fontFamily = FontFamily.Monospace, fontSize = 11.sp, color = KshanaColors.Sky.copy(alpha = alpha), modifier = Modifier.width(92.dp))
                Column(Modifier.weight(1f)) {
                    Text(r.words.substringBefore(':'), style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Text.copy(alpha = alpha))
                    Text(r.words.substringAfter(": ", ""), style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted.copy(alpha = alpha))
                }
            }
        }
        Text(
            "This looks as far ahead as ${localText(t.horizon, YEAR_ONLY)}. Everything is worked out on this phone. Nothing leaves it.",
            style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted, modifier = Modifier.padding(top = 8.dp),
        )
    }
}

internal val YEAR_ONLY: java.time.format.DateTimeFormatter = java.time.format.DateTimeFormatter.ofPattern("uuuu")

@Composable
private fun ChapterRow(c: ChapterRecord, t: TimelineRecord, open: Boolean, onToggle: () -> Unit, onLife: (String) -> Unit) {
    val past = c.end.seconds() < t.at.seconds()
    val alpha = if (past) PAST_ALPHA else 1f
    val col = bandColor(c.band)
    Column(
        Modifier.fillMaxWidth().clip(RoundedCornerShape(12.dp))
            .background(if (c.current) KshanaColors.Gold.copy(alpha = 0.10f) else KshanaColors.Surface)
            .then(if (c.current) Modifier.border(1.dp, KshanaColors.Gold, RoundedCornerShape(12.dp)) else Modifier)
            .clickable(onClick = onToggle)
            .padding(12.dp),
        verticalArrangement = Arrangement.spacedBy(4.dp),
    ) {
        Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
            Text(chapterShort(c.lord), fontFamily = FontFamily.Serif, fontSize = 17.sp, color = KshanaColors.Text.copy(alpha = alpha), modifier = Modifier.weight(1f))
            Text("age ${c.age.toInt()} – ${(c.age + yearsBetween(c.start, c.end)).toInt()}", fontFamily = FontFamily.Monospace, fontSize = 11.sp, color = KshanaColors.Muted.copy(alpha = alpha))
            Spacer(Modifier.width(10.dp))
            Text(c.tone.toInt().toString(), fontFamily = FontFamily.Monospace, fontSize = 16.sp, color = col.copy(alpha = alpha))
        }
        Box(Modifier.fillMaxWidth().height(4.dp).clip(RoundedCornerShape(2.dp)).background(KshanaColors.Line)) {
            Box(Modifier.fillMaxWidth((c.tone / 100.0).toFloat().coerceIn(0.02f, 1f)).height(4.dp).background(col.copy(alpha = alpha)))
        }
        Text("${dayMon(c.start)} ${localText(c.start, YEAR_ONLY)} – ${dayMon(c.end)} ${localText(c.end, YEAR_ONLY)}${if (c.current) "  · now" else ""}", fontFamily = FontFamily.Monospace, fontSize = 11.sp, color = (if (c.current) KshanaColors.Gold else KshanaColors.Sky).copy(alpha = alpha))
        Text(c.words, style = MaterialTheme.typography.bodyMedium, color = KshanaColors.TextSoft.copy(alpha = alpha))
        if (open) {
            HorizontalDivider(color = KshanaColors.LineSoft)
            for ((i, name) in LIFE_ORDER.withIndex()) {
                val v = c.lives.getOrNull(i) ?: 50.0
                Row(Modifier.fillMaxWidth().clickable { onLife(name) }.padding(vertical = 2.dp), verticalAlignment = Alignment.CenterVertically) {
                    Text("${lifeGlyph(name)} $name", style = MaterialTheme.typography.bodySmall, color = KshanaColors.TextSoft, modifier = Modifier.width(96.dp))
                    Box(Modifier.weight(1f).height(5.dp).clip(RoundedCornerShape(2.dp)).background(KshanaColors.Line)) {
                        Box(Modifier.fillMaxWidth((v / 100.0).toFloat().coerceIn(0.02f, 1f)).height(5.dp).background(scoreColor(v)))
                    }
                    Spacer(Modifier.width(8.dp))
                    Text(v.toInt().toString(), fontFamily = FontFamily.Monospace, fontSize = 11.sp, color = scoreColor(v), modifier = Modifier.width(24.dp))
                }
            }
        }
    }
}

private fun yearsBetween(a: uniffi.kshana_ffi.ExactInstant, b: uniffi.kshana_ffi.ExactInstant): Double =
    (b.seconds() - a.seconds()) / (365.25 * 86_400.0)
