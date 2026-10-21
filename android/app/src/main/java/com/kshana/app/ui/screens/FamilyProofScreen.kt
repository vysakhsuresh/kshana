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
import androidx.compose.runtime.mutableIntStateOf
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
import com.kshana.app.data.CityClocks
import com.kshana.app.data.EngineSettings
import com.kshana.app.data.FamilyStore
import com.kshana.app.data.Place
import com.kshana.app.data.ProfileStore
import com.kshana.app.engine.EngineProvider
import com.kshana.app.engine.Instants
import com.kshana.app.ui.sky.KshanaFonts
import com.kshana.app.ui.theme.KshanaColors
import com.kshana.app.ui.theme.glass
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import uniffi.kshana_ffi.FamilyMemberInput
import uniffi.kshana_ffi.FamilyProofRecord
import uniffi.kshana_ffi.FamilyProofResult

/** The relations a family chart can be read through, as the person picks them here. */
private val PROOF_RELATIONS = listOf("Father", "Mother", "Spouse", "Partner", "Child", "Sibling", "Friend")

/** The relation stored for a name, or null when none has been chosen yet. */
private fun storedRelation(context: android.content.Context, name: String): String? =
    runCatching {
        val o = org.json.JSONObject(ProfileStore.prefs(context).getString("partners.relations.json", "{}") ?: "{}")
        if (o.has(name)) o.getString(name) else null
    }.getOrNull()

/** Whether the person has said which side of a match they are; null when never set. */
private fun selfIsMale(context: android.content.Context): Boolean? =
    if (ProfileStore.prefs(context).contains("partners.selfIsGroom")) ProfileStore.selfIsGroom(context) else null

/** One or two letters for a name's circle. */
private fun initials(name: String): String {
    val parts = name.trim().split(Regex("\\s+")).filter { it.isNotEmpty() }
    return when {
        parts.isEmpty() -> "·"
        parts.size == 1 -> parts[0].take(1).uppercase()
        else -> (parts[0].take(1) + parts[1].take(1)).uppercase()
    }
}

/** The process keeps the last reading so coming back is instant. */
private object FamilyProofCache {
    var key: String? = null
    var result: FamilyProofResult? = null
}

/**
 * Family proof: the times of the coming year that more than one chart in the family
 * points to. A child's wedding shows in the parents' charts too, a father's change of
 * work in his children's; when the charts agree, the reading is much stronger. Each
 * window shows who agrees, as circles of initials. Hard times come with what helps.
 * With no family added, the page invites adding them.
 */
@Composable
fun FamilyProofScreen(
    profile: BirthProfile,
    place: Place,
    settings: EngineSettings,
    onBack: () -> Unit,
    onAddFamily: () -> Unit,
    onRemedy: () -> Unit,
) {
    val context = LocalContext.current
    val expert = com.kshana.app.data.Expert.on(context)
    var relVersion by remember { mutableIntStateOf(0) }
    val people = remember {
        (ProfileStore.partners(context) + FamilyStore.family(context))
            .filter { it.name != profile.name }
            .distinctBy { it.name }
    }
    val relations = remember(relVersion) { people.associate { it.name to storedRelation(context, it.name) } }
    val unplaced = people.filter { relations[it.name] == null }
    val readable = people.filter { relations[it.name] != null }
    var result by remember { mutableStateOf<FamilyProofResult?>(null) }
    var error by remember { mutableStateOf<String?>(null) }
    val zone = remember(place) { runCatching { CityClocks.of(place.zoneId) }.getOrDefault(java.time.ZoneId.systemDefault()) }

    val key = "${profile.name}|${profile.birthInstant().taiSeconds}|${readable.joinToString(",") { it.name + ":" + relations[it.name] + ":" + it.birthInstant().taiSeconds }}|${java.time.LocalDate.now()}|${settings.ayanamsa}|${settings.houseSystem}"
    LaunchedEffect(key) {
        if (readable.isEmpty()) { result = null; return@LaunchedEffect }
        if (FamilyProofCache.key == key) { result = FamilyProofCache.result; return@LaunchedEffect }
        result = null
        error = null
        runCatching {
            val engine = EngineProvider.get(context)
            val inputs = readable.map { FamilyMemberInput(it.name, relations[it.name] ?: "Other", it.birthInstant(), it.latitude, it.longitude) }
            val r = withContext(Dispatchers.Default) {
                engine.familyProof(profile.birthInstant(), profile.latitude, profile.longitude, selfIsMale(context), inputs, place.latitude, place.longitude, settings.houseSystem, settings.ayanamsa, Instants.now(), 12u)
            }
            FamilyProofCache.key = key
            FamilyProofCache.result = r
            result = r
        }.onFailure { error = it.message }
    }

    Column(
        Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(horizontal = 24.dp).padding(top = 44.dp, bottom = 32.dp),
        verticalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        Text("‹ BACK", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold, modifier = Modifier.clickable(onClick = onBack).padding(vertical = 4.dp))
        Text("FAMILY PROOF", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold)
        Text("When the family agrees", style = MaterialTheme.typography.displayMedium, color = KshanaColors.Text)
        Text(
            "A child's wedding shows in the parents' charts too, and a father's change of work in his children's. These are the times in the next year that more than one chart in your family points to.",
            style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted,
        )

        if (people.isEmpty()) {
            EmptyFamily(onAddFamily)
            return@Column
        }

        // Anyone whose place in the family is not known yet: ask, once, in a tap.
        for (p in unplaced) {
            Column(Modifier.fillMaxWidth().glass(14.dp).padding(14.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
                Row(verticalAlignment = Alignment.CenterVertically) {
                    Initials(initials(p.name), lit = false)
                    Spacer(Modifier.width(10.dp))
                    Text("Who is ${p.name.substringBefore(' ')} to you?", fontFamily = FontFamily.Serif, fontSize = 17.sp, color = KshanaColors.Text)
                }
                Row(Modifier.horizontalScroll(rememberScrollState()), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                    for (rel in PROOF_RELATIONS) Chip(rel.uppercase(), selected = false, onClick = {
                        ProfileStore.saveRelation(context, p.name, rel)
                        relVersion++
                    })
                }
            }
        }

        ErrorLine(error)
        if (readable.isEmpty()) {
            if (unplaced.isEmpty()) EmptyFamily(onAddFamily)
            return@Column
        }
        val r = result ?: run { if (error == null) Working("reading every chart in the family against the others…"); return@Column }
        val nameOf: (String) -> String = { n -> n.ifEmpty { profile.name } }

        Text(r.plain, fontFamily = KshanaFonts.serif, fontSize = 18.sp, lineHeight = 25.sp, color = KshanaColors.TextSoft)

        if (r.aboutYou.isNotEmpty()) SectionLabel("Your year, confirmed by family")
        for (w in r.aboutYou) ProofCard(w, nameOf, zone, expert, onRemedy)

        if (r.aboutFamily.isNotEmpty()) SectionLabel("What your chart says about them")
        for (w in r.aboutFamily) ProofCard(w, nameOf, zone, expert, onRemedy)

        if (r.aboutYou.isEmpty() && r.aboutFamily.isEmpty()) {
            Text("Each chart tells its own story for now. Come back as the months turn; adding more of the family gives the charts more to agree on.", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.TextSoft)
        }
        if (r.notRead.isNotEmpty()) {
            Text("Friends are not read here: ${r.notRead.joinToString(", ")}. Only family charts confirm each other.", style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted)
        }
        Spacer(Modifier.height(4.dp))
        Door("Add or change family", onClick = onAddFamily)
        Text("Agreement makes a time much more likely, never certain. Use it to prepare, not to fear.", style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted)
    }
}

@Composable
private fun EmptyFamily(onAddFamily: () -> Unit) {
    Column(Modifier.fillMaxWidth().glass(18.dp).padding(18.dp), verticalArrangement = Arrangement.spacedBy(10.dp)) {
        Row(horizontalArrangement = Arrangement.spacedBy((-8).dp)) {
            for (s in listOf("M", "F", "S")) Initials(s, lit = false)
        }
        Text("Your family's charts can confirm yours", fontFamily = FontFamily.Serif, fontSize = 19.sp, lineHeight = 25.sp, color = KshanaColors.Text)
        Text(
            "Add your parents, your partner, your children, your brothers and sisters. When their charts and yours point to the same weeks, you will see it here, with who agrees.",
            style = MaterialTheme.typography.bodyMedium, color = KshanaColors.TextSoft,
        )
        GoldButton("ADD FAMILY", onClick = onAddFamily, modifier = Modifier.fillMaxWidth())
        Text("Everything stays on this phone.", style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted)
    }
}

/** A circle with initials: gold when the person's chart is one that agrees. */
@Composable
private fun Initials(text: String, lit: Boolean, accent: Color = KshanaColors.Gold) {
    Box(
        Modifier.size(34.dp).clip(CircleShape)
            .background(if (lit) accent.copy(alpha = 0.16f) else KshanaColors.Surface)
            .border(1.dp, if (lit) accent else KshanaColors.Line, CircleShape),
        contentAlignment = Alignment.Center,
    ) {
        Text(text, fontFamily = KshanaFonts.serif, fontSize = 13.sp, color = if (lit) accent else KshanaColors.Muted)
    }
}

@Composable
private fun ProofCard(w: FamilyProofRecord, nameOf: (String) -> String, zone: java.time.ZoneId, expert: Boolean, onRemedy: () -> Unit) {
    val accent = if (w.isHard) KshanaColors.Ember else KshanaColors.Gold
    Column(Modifier.fillMaxWidth().glass(16.dp).padding(16.dp), verticalArrangement = Arrangement.spacedBy(6.dp)) {
        Text(
            "${localText(w.start, DATE_ONLY, zone)} – ${localText(w.end, DATE_ONLY, zone)}".uppercase(),
            style = MaterialTheme.typography.labelSmall, color = accent,
        )
        Text(w.headline, fontFamily = FontFamily.Serif, fontSize = 18.sp, lineHeight = 24.sp, color = KshanaColors.Text)
        // The person, then everyone whose chart agrees.
        Row(verticalAlignment = Alignment.CenterVertically) {
            Initials(initials(nameOf(w.person)), lit = true, accent = accent)
            Text("·", color = KshanaColors.Muted, modifier = Modifier.padding(horizontal = 6.dp))
            Row(horizontalArrangement = Arrangement.spacedBy((-8).dp)) {
                for (n in w.confirmedNames) Initials(initials(nameOf(n)), lit = true, accent = accent)
            }
            Spacer(Modifier.width(10.dp))
            Text(w.strengthWords, style = MaterialTheme.typography.bodySmall, color = accent, modifier = Modifier.weight(1f))
        }
        Text(w.detail, style = MaterialTheme.typography.bodyMedium, color = KshanaColors.TextSoft)
        if (w.isHard) {
            Text(w.remedyHint, style = MaterialTheme.typography.bodySmall, color = KshanaColors.Leaf)
            Text("WHAT HELPS ›", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold, modifier = Modifier.clickable(onClick = onRemedy).padding(vertical = 4.dp))
        } else if (w.remedyHint.isNotBlank()) {
            Text(w.remedyHint, style = MaterialTheme.typography.bodySmall, color = KshanaColors.Sky)
        }
        if (expert) for (e in w.expert) Text(e, style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted)
        WhereFrom(w.sourceKeys)
    }
}
