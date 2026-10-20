package com.kshana.app.ui.screens

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
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
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.kshana.app.data.BirthProfile
import com.kshana.app.data.EngineSettings
import com.kshana.app.data.Faith
import com.kshana.app.data.Lang
import com.kshana.app.data.Place
import com.kshana.app.data.RemedyPlans
import com.kshana.app.engine.EngineProvider
import com.kshana.app.engine.Instants
import com.kshana.app.ui.theme.KshanaColors
import com.kshana.app.ui.theme.glass
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import uniffi.kshana_ffi.CauseNodeRecord
import uniffi.kshana_ffi.RemedyTreeRecord
import uniffi.kshana_ffi.TreeRemedyRecord

internal object TreeCache {
    val trees = HashMap<String, RemedyTreeRecord>()
}

/**
 * What to do, as a tree: what is pressing on this part of your life, what it does to
 * you, until when, and under it everything that helps, starting with deeds anyone of any
 * faith can do. Faith practices show as the person chose in Settings.
 */
@Composable
fun RemedyTreeScreen(profile: BirthProfile, place: Place, settings: EngineSettings, onBack: () -> Unit, life: String? = null) {
    val context = LocalContext.current
    var chosen by remember { mutableStateOf(life) }
    var tree by remember { mutableStateOf<RemedyTreeRecord?>(null) }
    var error by remember { mutableStateOf<String?>(null) }
    var plansVersion by remember { mutableStateOf(0) }
    val faith = remember { Faith.get(context) }
    val lang = Lang.code
    val key = "${profile.birthInstant().taiSeconds}|${place.name}|$chosen|$lang|${java.time.LocalDate.now()}|${com.kshana.app.data.Focus.lives}"
    LaunchedEffect(key) {
        tree = TreeCache.trees[key]
        if (tree != null) return@LaunchedEffect
        try {
            val engine = EngineProvider.get(context)
            val t = withContext(Dispatchers.Default) {
                engine.remedyTree(profile.birthInstant(), profile.latitude, profile.longitude, place.latitude, place.longitude, settings.houseSystem, settings.ayanamsa, chosen, Instants.now(), lang, com.kshana.app.data.Focus.lives)
            }
            TreeCache.trees[key] = t
            tree = t
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
            Text("WHAT TO DO", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold)
            Text("BACK", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Muted, modifier = Modifier.clickable(onClick = onBack).padding(12.dp))
        }
        Text("What will help", style = MaterialTheme.typography.displayMedium, color = KshanaColors.Text)

        PlansCard(plansVersion) { plansVersion++ }

        Row(Modifier.fillMaxWidth().horizontalScroll(rememberScrollState()), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            Chip("WHAT WEIGHS MOST", selected = chosen == null, onClick = { chosen = null; tree = null })
            for (l in LIFE_ORDER) Chip("${lifeGlyph(l)} ${lifeName(l).uppercase()}", selected = chosen == l, onClick = { chosen = l; tree = null })
        }
        ErrorLine(error)
        val t = tree
        if (t == null) {
            if (error == null) Working("finding what will help…")
            return@Column
        }
        Text(t.headline, fontFamily = FontFamily.Serif, fontSize = 20.sp, lineHeight = 28.sp, color = KshanaColors.Text)
        if (t.causes.isEmpty()) {
            Text("Nothing is pressing on this part of your life now. Keep doing what you do.", style = MaterialTheme.typography.bodyLarge, color = KshanaColors.TextSoft)
        }
        for ((i, c) in t.causes.withIndex()) CauseCard(i, c, faith, onPlan = { plansVersion++ })
        Text(
            "Deeds come first because they work for everyone, of any faith or none. Choose which faith practices to see in Settings › Remedies for you. None is ever required, and no remedy replaces a doctor, a lawyer or a counsellor.",
            style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted,
        )
    }
}

/** One cause: the trunk of the tree, with its branches of help. */
@Composable
private fun CauseCard(index: Int, c: CauseNodeRecord, faith: String, onPlan: () -> Unit) {
    var open by remember { mutableStateOf(index == 0) }
    Column(Modifier.fillMaxWidth().glass(18.dp).padding(16.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
        Row(Modifier.fillMaxWidth().clickable { open = !open }, verticalAlignment = Alignment.CenterVertically) {
            Column(Modifier.weight(1f)) {
                Text("WHY IT FEELS THIS WAY · ${c.strengthWords.uppercase()}", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Ember)
                Text(c.title, fontFamily = FontFamily.Serif, fontSize = 20.sp, lineHeight = 26.sp, color = KshanaColors.Text)
            }
            Text(if (open) "−" else "+", fontSize = 22.sp, color = KshanaColors.Gold)
        }
        Text(c.whatItDoes, style = MaterialTheme.typography.bodyLarge, color = KshanaColors.TextSoft)
        if (c.untilWords.isNotBlank()) Text(c.untilWords.replaceFirstChar { it.uppercase() }, style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Sky)
        if (!open) return@Column
        Text("WHAT HELPS", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold, modifier = Modifier.padding(top = 4.dp))
        for (b in c.branches) {
            if (!Faith.shows(faith, b.key) || b.items.isEmpty()) continue
            // A branch: a gold thread down the left, its title, its intro, its remedies.
            Row(Modifier.fillMaxWidth().height(androidx.compose.foundation.layout.IntrinsicSize.Min)) {
                Box(Modifier.width(2.dp).fillMaxHeight().background(if (b.key == "tradition") KshanaColors.Line else KshanaColors.Gold.copy(alpha = 0.5f)))
                Spacer(Modifier.width(12.dp))
                Column(Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(6.dp)) {
                    Text(b.title, style = MaterialTheme.typography.titleMedium, color = if (b.key == "tradition") KshanaColors.TextSoft else KshanaColors.Gold)
                    if (b.intro.isNotBlank()) Text(b.intro, style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted)
                    for (r in b.items) RemedyItem(r, c.title, onPlan)
                }
            }
            Spacer(Modifier.height(4.dp))
        }
        HowThisHelps(c.howItHelps)
    }
}

/**
 * The chart reason behind a set of remedies, closed until the person asks: which planet
 * presses this part of life, and why serving its people, giving on its day and praying
 * eases it.
 */
@Composable
internal fun HowThisHelps(text: String) {
    if (text.isBlank()) return
    var show by remember { mutableStateOf(false) }
    Column(
        Modifier.fillMaxWidth().clip(RoundedCornerShape(12.dp)).background(KshanaColors.Night.copy(alpha = 0.45f)).clickable { show = !show }.padding(12.dp),
        verticalArrangement = Arrangement.spacedBy(6.dp),
    ) {
        Text(if (show) "HOW THIS HELPS −" else "HOW THIS HELPS +  ·  the reason in the chart", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold)
        androidx.compose.animation.AnimatedVisibility(show) {
            Text(text, style = MaterialTheme.typography.bodyMedium, lineHeight = 22.sp, color = KshanaColors.TextSoft)
        }
    }
}

@Composable
private fun RemedyItem(r: TreeRemedyRecord, forWhat: String, onPlan: () -> Unit) {
    val context = LocalContext.current
    var open by remember { mutableStateOf(false) }
    var started by remember { mutableStateOf(false) }
    Column(
        Modifier.fillMaxWidth().clip(RoundedCornerShape(12.dp)).background(KshanaColors.Night.copy(alpha = 0.45f)).clickable { open = !open }.padding(12.dp),
        verticalArrangement = Arrangement.spacedBy(4.dp),
    ) {
        Text(r.title, style = MaterialTheme.typography.bodyLarge, color = KshanaColors.Text)
        Text(r.why, style = MaterialTheme.typography.bodyMedium, color = KshanaColors.TextSoft)
        Text(listOf(r.day, r.effort).filter { it.isNotBlank() && it !in setOf("any day", "ഏതു ദിവസവും", "कोई भी दिन") }.joinToString(" · "), style = MaterialTheme.typography.bodySmall, color = KshanaColors.Sky)
        if (open) {
            if (r.how.isNotBlank()) Text(r.how, style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Text)
            r.mantra?.takeIf { it.isNotBlank() }?.let { Text(it, fontFamily = FontFamily.Serif, fontSize = 17.sp, color = KshanaColors.Gold) }
            Row(Modifier.horizontalScroll(rememberScrollState()), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                QuietButton(if (started) "STARTED ✓" else "DO IT FOR 21 DAYS", onClick = {
                    RemedyPlans.start(context, r.title, r.how, forWhat, 21, weekdayOf(r.day))
                    started = true
                    onPlan()
                })
                if (r.kind in setOf("Anna", "Daana", "Jeeva", "Vastra", "giving")) {
                    QuietButton("GIVE NOW", onClick = { runCatching { context.startActivity(RemedyPlans.giveIntent(context, r.title)) } })
                }
                r.mantra?.takeIf { it.isNotBlank() }?.let { m ->
                    QuietButton("CHANT ×108", onClick = { com.kshana.app.system.Chant.start(context, m) })
                }
                QuietButton("READ ALOUD", onClick = { com.kshana.app.system.Speak.say(context, "${r.title}. ${r.why}. ${r.how}", Lang.code) })
            }
        } else {
            Text("HOW, AND START IT ›", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold)
        }
    }
}

/** "Saturday" (in any of the three languages' English key) → ISO day number, else every day. */
private fun weekdayOf(day: String): Int {
    val d = day.lowercase()
    return listOf("monday", "tuesday", "wednesday", "thursday", "friday", "saturday", "sunday").indexOfFirst { it in d }.let { if (it < 0) 0 else it + 1 }
}

/** The practices the person has taken on, with today's tick and the run so far. */
@Composable
private fun PlansCard(version: Int, onChange: () -> Unit) {
    val context = LocalContext.current
    val plans = remember(version) { RemedyPlans.active(context) }
    if (plans.isEmpty()) return
    val today = java.time.LocalDate.now()
    Column(Modifier.fillMaxWidth().glass(16.dp).padding(14.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
        Text("WHAT YOU HAVE TAKEN ON", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold)
        for (p in plans) {
            val due = p.dueOn(today)
            val done = today in p.done
            Row(verticalAlignment = Alignment.CenterVertically) {
                Column(Modifier.weight(1f)) {
                    Text(p.title, style = MaterialTheme.typography.bodyLarge, color = KshanaColors.Text)
                    Text("Day ${p.dayNumber()} of ${p.days} · done ${p.doneCount()} of ${p.dueCount()}", style = MaterialTheme.typography.bodySmall, color = KshanaColors.Sky)
                }
                if (due) Chip(if (done) "✓ TODAY" else "DONE TODAY?", selected = done, onClick = { RemedyPlans.toggleToday(context, p.id); onChange() })
                else Text("not today", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Muted)
            }
            Text("STOP", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Muted, modifier = Modifier.clickable { RemedyPlans.stop(context, p.id); onChange() })
        }
        Text("Your sunrise reading and the home-screen widget remind you of today's practice.", style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted)
    }
}
