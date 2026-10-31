/*
    LibrePods - AirPods liberated from Apple’s ecosystem
    Copyright (C) 2025 LibrePods contributors

    This program is free software: you can redistribute it and/or modify
    it under the terms of the GNU General Public License as published by
    the Free Software Foundation, either version 3 of the License, or
    any later version.

    This program is distributed in the hope that it will be useful,
    but WITHOUT ANY WARRANTY; without even the implied warranty of
    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
    GNU General Public License for more details.

    You should have received a copy of the GNU General Public License
    along with this program.  If not, see <https://www.gnu.org/licenses/>.
*/

package me.kavishdevar.librepods.presentation.components

import kotlinx.coroutines.delay
import androidx.compose.runtime.remember
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.animation.Crossfade
import androidx.compose.animation.core.tween
import androidx.compose.animation.core.spring
import androidx.compose.animation.core.Animatable
import androidx.annotation.DrawableRes
import androidx.compose.animation.animateColorAsState
import androidx.compose.animation.core.animateDpAsState
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.RowScope
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.selection.selectable
import androidx.compose.foundation.selection.toggleable
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.RadioButton
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Shape
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import me.kavishdevar.librepods.R
import me.kavishdevar.librepods.data.Battery
import me.kavishdevar.librepods.data.BatteryStatus

/*
 * Material 3 Expressive building blocks for the redesigned screens: grouped settings lists
 * (Android 16 style), a connected listening-mode button group, battery tiles and toggle tiles.
 */

private val GroupOuter = 24.dp
private val GroupInner = 4.dp

/** Card colour of grouped rows, sitting on the surfaceContainer page background. */
@Composable
fun cardColor(): Color = MaterialTheme.colorScheme.surfaceBright

@Composable
fun SectionHeader(text: String, modifier: Modifier = Modifier) {
    Text(
        text = text,
        style = MaterialTheme.typography.labelLarge,
        color = MaterialTheme.colorScheme.primary,
        modifier = modifier.padding(start = 8.dp, top = 24.dp, bottom = 10.dp)
    )
}

/** Rows separated by 2dp gaps; outer corners rounded, inner corners tight. Null rows are skipped. */
@Composable
fun SettingsGroup(
    modifier: Modifier = Modifier,
    vararg rows: (@Composable (Modifier) -> Unit)?
) {
    val visible = rows.filterNotNull()
    Column(modifier = modifier, verticalArrangement = Arrangement.spacedBy(2.dp)) {
        visible.forEachIndexed { index, row ->
            val top = if (index == 0) GroupOuter else GroupInner
            val bottom = if (index == visible.lastIndex) GroupOuter else GroupInner
            row(
                Modifier
                    .fillMaxWidth()
                    .clip(RoundedCornerShape(top, top, bottom, bottom))
                    .background(cardColor())
            )
        }
    }
}

enum class LeadTone { Primary, Secondary, Tertiary }

@Composable
fun SettingsRow(
    modifier: Modifier,
    title: String,
    subtitle: String? = null,
    @DrawableRes icon: Int? = null,
    tone: LeadTone = LeadTone.Primary,
    onClick: (() -> Unit)? = null,
    enabled: Boolean = true,
    wip: Boolean = false,
    trailing: (@Composable RowScope.() -> Unit)? = null,
) {
    val active = enabled && !wip
    Row(
        modifier = modifier
            .then(if (onClick != null && active) Modifier.clickable(onClick = onClick) else Modifier)
            .heightIn(min = 64.dp)
            .padding(horizontal = 16.dp, vertical = 12.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(16.dp)
    ) {
        Row(
            modifier = Modifier
                .weight(1f)
                .alpha(if (active) 1f else 0.5f),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(16.dp)
        ) {
            if (icon != null) {
                val (bg, fg) = when (tone) {
                    LeadTone.Primary -> MaterialTheme.colorScheme.primaryContainer to MaterialTheme.colorScheme.onPrimaryContainer
                    LeadTone.Secondary -> MaterialTheme.colorScheme.secondaryContainer to MaterialTheme.colorScheme.onSecondaryContainer
                    LeadTone.Tertiary -> MaterialTheme.colorScheme.tertiaryContainer to MaterialTheme.colorScheme.onTertiaryContainer
                }
                Box(
                    modifier = Modifier.size(40.dp).clip(CircleShape).background(bg),
                    contentAlignment = Alignment.Center
                ) {
                    Icon(painterResource(icon), contentDescription = null, tint = fg, modifier = Modifier.size(22.dp))
                }
            }
            Column(modifier = Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(2.dp)) {
                Text(title, style = MaterialTheme.typography.bodyLarge, color = MaterialTheme.colorScheme.onSurface)
                if (subtitle != null) {
                    Text(subtitle, style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
                }
            }
        }
        if (wip) WipBadge()
        if (trailing != null) trailing()
        else if (onClick != null && !wip) {
            Icon(
                painterResource(R.drawable.ic_ui_chevron_right),
                contentDescription = null,
                tint = MaterialTheme.colorScheme.onSurfaceVariant
            )
        }
    }
}

@Composable
fun SwitchRow(
    modifier: Modifier,
    title: String,
    subtitle: String? = null,
    checked: Boolean,
    onCheckedChange: (Boolean) -> Unit,
    @DrawableRes icon: Int? = null,
    tone: LeadTone = LeadTone.Primary,
    enabled: Boolean = true,
) {
    SettingsRow(
        modifier = modifier.toggleable(
            value = checked,
            enabled = enabled,
            role = Role.Switch,
            onValueChange = onCheckedChange
        ),
        title = title,
        subtitle = subtitle,
        icon = icon,
        tone = tone,
        enabled = enabled,
    ) {
        Switch(checked = checked, onCheckedChange = null, enabled = enabled)
    }
}

@Composable
fun RadioRow(
    modifier: Modifier,
    title: String,
    subtitle: String? = null,
    selected: Boolean,
    onClick: () -> Unit,
) {
    SettingsRow(
        modifier = modifier.selectable(selected = selected, role = Role.RadioButton, onClick = onClick),
        title = title,
        subtitle = subtitle,
    ) {
        RadioButton(selected = selected, onClick = null)
    }
}

@Composable
fun WipBadge() {
    Text(
        text = stringResource(R.string.ui_wip),
        style = MaterialTheme.typography.labelSmall,
        fontWeight = FontWeight.SemiBold,
        color = MaterialTheme.colorScheme.onSurfaceVariant,
        modifier = Modifier
            .clip(RoundedCornerShape(11.dp))
            .background(MaterialTheme.colorScheme.surfaceContainerHighest)
            .padding(horizontal = 8.dp, vertical = 3.dp)
    )
}

data class ListeningModeOption(val mode: Int, val label: String, @DrawableRes val icon: Int, @DrawableRes val iconSelected: Int)

/** Listening modes in AACP order; values match ControlCommandIdentifiers.LISTENING_MODE. */
@Composable
fun listeningModeOptions(showOff: Boolean, showAdaptive: Boolean): List<ListeningModeOption> = listOfNotNull(
    if (showOff) ListeningModeOption(1, stringResource(R.string.ui_mode_off), R.drawable.ic_w_noise_control_off, R.drawable.ic_w_noise_control_off_fill) else null,
    ListeningModeOption(3, stringResource(R.string.ui_mode_transparency), R.drawable.ic_w_hearing, R.drawable.ic_w_hearing_fill),
    if (showAdaptive) ListeningModeOption(4, stringResource(R.string.ui_mode_adaptive), R.drawable.ic_w_auto_awesome, R.drawable.ic_w_auto_awesome_fill) else null,
    ListeningModeOption(2, stringResource(R.string.ui_mode_anc), R.drawable.ic_w_noise_control_on, R.drawable.ic_w_noise_control_on_fill),
)

/** Connected button group: the selected segment morphs to a full pill. */
@Composable
fun ListeningModeGroup(
    options: List<ListeningModeOption>,
    selected: Int,
    onSelect: (Int) -> Unit,
    modifier: Modifier = Modifier,
    height: Dp = 64.dp,
    showLabels: Boolean = true,
) {
    // Optimistic: the tapped mode lights up at once and holds until the AirPods report back
    // (or a few seconds pass, in which case we fall back to what they actually report).
    var pending by remember { mutableStateOf<Int?>(null) }
    LaunchedEffect(selected) { if (selected == pending) pending = null }
    LaunchedEffect(pending) { if (pending != null) { delay(3000); pending = null } }
    val shown = pending ?: selected

    Row(modifier = modifier.fillMaxWidth().height(height), horizontalArrangement = Arrangement.spacedBy(2.dp)) {
        options.forEachIndexed { index, option ->
            val isSelected = option.mode == shown
            val full = height / 2
            val spring = spring<Dp>(dampingRatio = 0.6f, stiffness = 500f)
            val start by animateDpAsState(if (isSelected || index == 0) full else 10.dp, spring, label = "start")
            val end by animateDpAsState(if (isSelected || index == options.lastIndex) full else 10.dp, spring, label = "end")
            val bg by animateColorAsState(
                if (isSelected) MaterialTheme.colorScheme.primary else MaterialTheme.colorScheme.surfaceContainerHighest,
                label = "bg"
            )
            val fg by animateColorAsState(
                if (isSelected) MaterialTheme.colorScheme.onPrimary else MaterialTheme.colorScheme.onSurfaceVariant,
                label = "fg"
            )
            // A short squash on selection, like the expressive button group.
            val scale = remember { Animatable(1f) }
            LaunchedEffect(isSelected) {
                if (isSelected) {
                    scale.animateTo(0.92f, tween(70))
                    scale.animateTo(1f, spring(dampingRatio = 0.45f, stiffness = 600f))
                }
            }
            Column(
                modifier = Modifier
                    .weight(1f)
                    .fillMaxHeight()
                    .graphicsLayer { scaleX = scale.value; scaleY = scale.value }
                    .clip(RoundedCornerShape(start, end, end, start))
                    .background(bg)
                    .selectable(selected = isSelected, role = Role.RadioButton) {
                        if (option.mode != shown) {
                            pending = option.mode
                            onSelect(option.mode)
                        }
                    }
                    .padding(horizontal = 4.dp),
                horizontalAlignment = Alignment.CenterHorizontally,
                verticalArrangement = Arrangement.Center
            ) {
                Crossfade(isSelected, animationSpec = tween(180), label = "icon") { sel ->
                    Icon(
                        painterResource(if (sel) option.iconSelected else option.icon),
                        contentDescription = if (showLabels) null else option.label,
                        tint = fg
                    )
                }
                if (showLabels) {
                    Spacer(Modifier.height(4.dp))
                    Text(
                        option.label,
                        color = fg,
                        style = MaterialTheme.typography.labelMedium,
                        maxLines = 1,
                        textAlign = TextAlign.Center
                    )
                }
            }
        }
    }
}

@Composable
fun ToggleTile(
    label: String,
    @DrawableRes icon: Int,
    checked: Boolean,
    onCheckedChange: (Boolean) -> Unit,
    modifier: Modifier = Modifier,
    enabled: Boolean = true,
) {
    val bg by animateColorAsState(if (checked) MaterialTheme.colorScheme.primary else cardColor(), label = "tile")
    val fg = if (checked) MaterialTheme.colorScheme.onPrimary else MaterialTheme.colorScheme.onSurface
    Column(
        modifier = modifier
            .clip(RoundedCornerShape(24.dp))
            .background(bg)
            .toggleable(value = checked, enabled = enabled, role = Role.Switch, onValueChange = onCheckedChange)
            .heightIn(min = 80.dp)
            .padding(16.dp)
            .alpha(if (enabled) 1f else 0.5f),
        verticalArrangement = Arrangement.SpaceBetween
    ) {
        Icon(painterResource(icon), contentDescription = null, tint = fg)
        Spacer(Modifier.height(16.dp))
        Text(label, color = fg, style = MaterialTheme.typography.labelLarge)
    }
}

fun Battery.isCharging() = status == BatteryStatus.CHARGING || status == BatteryStatus.OPTIMIZED_CHARGING

/** Thick rounded battery bar. */
@Composable
fun BatteryBar(level: Int, modifier: Modifier = Modifier, charging: Boolean = false, height: Dp = 10.dp) {
    val color = when {
        level <= 10 && !charging -> MaterialTheme.colorScheme.error
        charging -> MaterialTheme.colorScheme.tertiary
        else -> MaterialTheme.colorScheme.primary
    }
    val shape: Shape = RoundedCornerShape(height / 2)
    Box(modifier.fillMaxWidth().height(height).clip(shape).background(MaterialTheme.colorScheme.surfaceContainerHighest)) {
        Box(Modifier.fillMaxWidth(level.coerceIn(0, 100) / 100f).fillMaxHeight().clip(shape).background(color))
    }
}

@Composable
fun BatteryTile(label: String, battery: Battery, modifier: Modifier = Modifier) {
    Column(
        modifier = modifier
            .clip(RoundedCornerShape(24.dp))
            .background(cardColor())
            .padding(start = 14.dp, end = 14.dp, top = 14.dp, bottom = 16.dp),
        verticalArrangement = Arrangement.spacedBy(10.dp)
    ) {
        Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(4.dp)) {
            Text(label, style = MaterialTheme.typography.labelLarge, color = MaterialTheme.colorScheme.onSurfaceVariant)
            if (battery.isCharging()) {
                Icon(
                    painterResource(R.drawable.ic_w_bolt_fill), contentDescription = null,
                    tint = MaterialTheme.colorScheme.tertiary, modifier = Modifier.size(16.dp)
                )
            }
        }
        Text("${battery.level}%", style = MaterialTheme.typography.titleLarge)
        BatteryBar(battery.level, charging = battery.isCharging())
    }
}

@Composable
fun ChoiceChip(label: String, selected: Boolean, onClick: () -> Unit, @DrawableRes icon: Int) {
    val bg = if (selected) MaterialTheme.colorScheme.secondaryContainer else Color.Transparent
    val fg = if (selected) MaterialTheme.colorScheme.onSecondaryContainer else MaterialTheme.colorScheme.onSurfaceVariant
    Row(
        modifier = Modifier
            .height(32.dp)
            .clip(RoundedCornerShape(8.dp))
            .background(bg)
            .then(
                if (selected) Modifier
                else Modifier.border(1.dp, MaterialTheme.colorScheme.outlineVariant, RoundedCornerShape(8.dp))
            )
            .toggleable(value = selected, role = Role.Checkbox, onValueChange = { onClick() })
            .padding(horizontal = 12.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(6.dp)
    ) {
        Icon(painterResource(if (selected) R.drawable.ic_ui_check else icon), contentDescription = null, tint = fg, modifier = Modifier.size(18.dp))
        Text(label, style = MaterialTheme.typography.labelLarge, color = fg)
    }
}

@Composable
fun HintText(text: String) {
    Text(
        text,
        style = MaterialTheme.typography.bodySmall,
        color = MaterialTheme.colorScheme.onSurfaceVariant,
        modifier = Modifier.padding(start = 8.dp, end = 8.dp, top = 10.dp)
    )
}

@Composable
fun HorizontalSpacer(width: Dp) = Spacer(Modifier.width(width))
