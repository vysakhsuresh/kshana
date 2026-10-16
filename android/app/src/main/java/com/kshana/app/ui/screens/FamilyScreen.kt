package com.kshana.app.ui.screens

import androidx.compose.foundation.clickable
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
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.KeyboardArrowRight
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.kshana.app.data.BirthProfile
import com.kshana.app.data.FamilyStore
import com.kshana.app.data.ProfileStore
import com.kshana.app.ui.theme.KshanaColors

/**
 * The people the app knows besides you. Adding or editing reuses the birth form; the
 * active profile is restored afterwards because the form saves whatever it casts.
 */
@Composable
fun FamilyScreen(onBack: () -> Unit) {
    val context = LocalContext.current
    var family by remember { mutableStateOf(FamilyStore.family(context)) }
    var adding by remember { mutableStateOf(false) }
    var editing by remember { mutableStateOf<BirthProfile?>(null) }
    var armed by remember { mutableStateOf<String?>(null) }

    if (adding || editing != null) {
        val previous = remember { ProfileStore.load(context) }
        val original = editing
        Column(Modifier.fillMaxSize()) {
            Row(Modifier.fillMaxWidth().padding(horizontal = 24.dp).padding(top = 48.dp), horizontalArrangement = Arrangement.SpaceBetween, verticalAlignment = Alignment.CenterVertically) {
                Text(if (original == null) "FAMILY · ADD" else "FAMILY · EDIT", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold)
                Text("CANCEL", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Muted, modifier = Modifier.clickable { adding = false; editing = null }.padding(12.dp))
            }
            BirthScreen(existing = original, onCast = { p ->
                if (previous != null) ProfileStore.save(context, previous)
                if (original != null && original.name != p.name) FamilyStore.remove(context, original.name)
                FamilyStore.upsert(context, p)
                family = FamilyStore.family(context)
                adding = false
                editing = null
            })
        }
        return
    }

    Column(
        Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(horizontal = 24.dp).padding(top = 48.dp, bottom = 32.dp),
        verticalArrangement = Arrangement.spacedBy(10.dp),
    ) {
        Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween, verticalAlignment = Alignment.CenterVertically) {
            Text("FAMILY", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold)
            Text("BACK", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Muted, modifier = Modifier.clickable(onClick = onBack).padding(12.dp))
        }
        Text("The people around you", style = MaterialTheme.typography.displayMedium, color = KshanaColors.Text)

        if (family.isEmpty()) {
            Text(
                "Add a partner, your children, your parents or a friend. Each birth is kept here so you can match charts and read their skies. Everything stays on this phone.",
                style = MaterialTheme.typography.bodyMedium, color = KshanaColors.TextSoft,
            )
        } else {
            Text("${family.size} ${if (family.size == 1) "person" else "people"} · tap a row to edit", style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted)
            Column(Modifier.fillMaxWidth()) {
                for (p in family) {
                    HorizontalDivider(color = KshanaColors.LineSoft)
                    Row(
                        Modifier.fillMaxWidth().heightIn(min = 56.dp).clickable { editing = p }.padding(vertical = 8.dp),
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        Column(Modifier.weight(1f)) {
                            Text(p.name, fontFamily = FontFamily.Serif, fontSize = 20.sp, color = KshanaColors.Text)
                            Text(p.birthLocal().format(DATE_TIME), fontFamily = FontFamily.Monospace, fontSize = 11.sp, color = KshanaColors.Sky)
                            Text(p.placeName, style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted)
                        }
                        QuietButton(
                            if (armed == p.name) "TAP AGAIN TO REMOVE" else "REMOVE",
                            onClick = {
                                if (armed == p.name) {
                                    FamilyStore.remove(context, p.name)
                                    family = FamilyStore.family(context)
                                    armed = null
                                } else {
                                    armed = p.name
                                }
                            },
                        )
                        Icon(Icons.Filled.KeyboardArrowRight, contentDescription = null, tint = KshanaColors.Gold, modifier = Modifier.padding(start = 8.dp))
                    }
                }
                HorizontalDivider(color = KshanaColors.LineSoft)
            }
        }

        Spacer(Modifier.height(8.dp))
        GoldButton("ADD A PERSON", onClick = { armed = null; adding = true }, modifier = Modifier.fillMaxWidth())
        Text("A person is stored by name; adding the same name again replaces the earlier entry.", style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted)
    }
}
