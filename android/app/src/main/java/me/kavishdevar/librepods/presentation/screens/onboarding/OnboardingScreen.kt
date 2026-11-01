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

package me.kavishdevar.librepods.presentation.screens.onboarding

import android.annotation.SuppressLint
import android.app.StatusBarManager
import android.appwidget.AppWidgetManager
import android.bluetooth.BluetoothDevice
import android.bluetooth.BluetoothManager
import android.content.ComponentName
import android.content.Context
import android.content.Intent
import android.graphics.drawable.Icon
import android.os.ParcelUuid
import android.provider.Settings
import androidx.activity.compose.BackHandler
import androidx.annotation.DrawableRes
import androidx.compose.animation.AnimatedContent
import androidx.compose.animation.core.RepeatMode
import androidx.compose.animation.core.animateFloat
import androidx.compose.animation.core.infiniteRepeatable
import androidx.compose.animation.core.rememberInfiniteTransition
import androidx.compose.animation.core.tween
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.slideInHorizontally
import androidx.compose.animation.slideOutHorizontally
import androidx.compose.animation.togetherWith
import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.safeDrawingPadding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Button
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.FilledTonalButton
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.LoadingIndicator
import androidx.compose.material3.MaterialShapes
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.toShape
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.rotate
import androidx.compose.ui.draw.scale
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.google.accompanist.permissions.ExperimentalPermissionsApi
import com.google.accompanist.permissions.isGranted
import com.google.accompanist.permissions.rememberMultiplePermissionsState
import com.google.accompanist.permissions.rememberPermissionState
import kotlinx.coroutines.delay
import me.kavishdevar.librepods.R
import me.kavishdevar.librepods.bluetooth.BluetoothConnectionManager
import me.kavishdevar.librepods.data.AirPodsMax
import me.kavishdevar.librepods.data.BatteryComponent
import me.kavishdevar.librepods.data.BatteryStatus
import me.kavishdevar.librepods.presentation.components.LeadTone
import me.kavishdevar.librepods.presentation.components.SectionHeader
import me.kavishdevar.librepods.presentation.components.SettingsGroup
import me.kavishdevar.librepods.presentation.components.SettingsRow
import me.kavishdevar.librepods.presentation.components.SwitchRow
import me.kavishdevar.librepods.presentation.theme.LibrePodsTheme
import me.kavishdevar.librepods.presentation.widgets.BatteryWidget
import me.kavishdevar.librepods.services.AirPodsQSService
import me.kavishdevar.librepods.services.AirPodsService
import me.kavishdevar.librepods.services.ServiceManager
import me.kavishdevar.librepods.services.cleanDeviceName
import me.kavishdevar.librepods.services.looksLikeAirPods
import me.kavishdevar.librepods.utils.XposedState
import me.kavishdevar.librepods.utils.bypassDeviceCheck
import me.kavishdevar.librepods.utils.isSupported

private const val STEP_WELCOME = 0
private const val STEP_PERMISSIONS = 1
private const val STEP_FIND = 2
private const val STEP_DONE = 3
private val AACP_UUID: ParcelUuid = ParcelUuid.fromString("74ec2172-0bad-4d01-8f77-997b2be0722a")

/**
 * First-run setup: welcome → permissions → find your AirPods → ready.
 * Unlike the old carousel it actually finds and connects a pair before finishing.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun OnboardingScreen(
    onOnboardingComplete: () -> Unit,
) {
    var step by rememberSaveable { mutableIntStateOf(STEP_WELCOME) }
    var chosenMac by rememberSaveable { mutableStateOf<String?>(null) }

    BackHandler(enabled = step > STEP_WELCOME) { step-- }

    LibrePodsTheme {
        Column(
            Modifier
                .fillMaxSize()
                .background(MaterialTheme.colorScheme.surfaceContainer)
                .safeDrawingPadding()
        ) {
            if (step > STEP_WELCOME) SetupTopBar(step, onBack = { step-- })
            AnimatedContent(
                targetState = step,
                transitionSpec = {
                    val dir = if (targetState > initialState) 1 else -1
                    (slideInHorizontally { it * dir / 4 } + fadeIn()) togetherWith
                        (slideOutHorizontally { -it * dir / 4 } + fadeOut())
                },
                label = "setup",
                modifier = Modifier.weight(1f)
            ) { s ->
                when (s) {
                    STEP_WELCOME -> WelcomeStep(onNext = { step = STEP_PERMISSIONS })
                    STEP_PERMISSIONS -> PermissionsStep(onNext = { step = STEP_FIND })
                    STEP_FIND -> FindStep(
                        onChosen = { chosenMac = it; step = STEP_DONE },
                        onSkip = onOnboardingComplete
                    )
                    else -> DoneStep(chosenMac, onFinish = onOnboardingComplete)
                }
            }
        }
    }
}

@Composable
private fun SetupTopBar(step: Int, onBack: () -> Unit) {
    Row(
        Modifier
            .fillMaxWidth()
            .height(56.dp)
            .padding(start = 4.dp, end = 24.dp),
        verticalAlignment = Alignment.CenterVertically
    ) {
        IconButton(onClick = onBack) {
            Icon(painterResource(R.drawable.ic_ui_arrow_back), contentDescription = stringResource(R.string.back))
        }
        Row(Modifier.weight(1f).padding(start = 8.dp), horizontalArrangement = Arrangement.spacedBy(6.dp)) {
            repeat(4) { i ->
                Box(
                    Modifier
                        .weight(1f)
                        .height(4.dp)
                        .clip(CircleShape)
                        .background(if (i <= step) MaterialTheme.colorScheme.primary else MaterialTheme.colorScheme.surfaceContainerHighest)
                )
            }
        }
    }
}

@Composable
private fun StepPage(
    title: String,
    subtitle: String,
    content: @Composable ColumnScope.() -> Unit,
    bottom: @Composable ColumnScope.() -> Unit,
) {
    Column(Modifier.fillMaxSize().padding(horizontal = 16.dp)) {
        Column(
            Modifier
                .weight(1f)
                .verticalScroll(rememberScrollState())
        ) {
            Text(
                title,
                style = MaterialTheme.typography.headlineLarge,
                modifier = Modifier.padding(start = 8.dp, end = 8.dp, top = 16.dp)
            )
            Text(
                subtitle,
                style = MaterialTheme.typography.bodyLarge,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                modifier = Modifier.padding(start = 8.dp, end = 8.dp, top = 8.dp, bottom = 8.dp)
            )
            content()
        }
        Column(Modifier.padding(top = 12.dp, bottom = 16.dp), verticalArrangement = Arrangement.spacedBy(8.dp), content = bottom)
    }
}

@Composable
private fun WideButton(text: String, onClick: () -> Unit, enabled: Boolean = true, tonal: Boolean = false, @DrawableRes icon: Int? = null) {
    val content: @Composable () -> Unit = {
        if (icon != null) {
            Icon(painterResource(icon), null, Modifier.size(20.dp))
            Spacer(Modifier.size(8.dp))
        }
        Text(text, style = MaterialTheme.typography.titleMedium)
    }
    if (tonal) FilledTonalButton(onClick, Modifier.fillMaxWidth().height(56.dp), enabled = enabled) { content() }
    else Button(onClick, Modifier.fillMaxWidth().height(56.dp), enabled = enabled) { content() }
}

// ---- 1 · Welcome ----

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun WelcomeStep(onNext: () -> Unit) {
    var showPrivacy by remember { mutableStateOf(false) }
    Column(Modifier.fillMaxSize().padding(horizontal = 24.dp, vertical = 16.dp)) {
        Column(
            Modifier.weight(1f).fillMaxWidth(),
            horizontalAlignment = Alignment.CenterHorizontally,
            verticalArrangement = Arrangement.Center
        ) {
            WelcomeHero()
            Spacer(Modifier.height(40.dp))
            Text(
                stringResource(R.string.app_name),
                style = MaterialTheme.typography.displayMedium,
                fontWeight = FontWeight.SemiBold,
                textAlign = TextAlign.Center
            )
            Spacer(Modifier.height(12.dp))
            Text(
                stringResource(R.string.setup_welcome_tagline),
                style = MaterialTheme.typography.bodyLarge,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                textAlign = TextAlign.Center,
                modifier = Modifier.padding(horizontal = 24.dp)
            )
        }
        Row(
            Modifier
                .fillMaxWidth()
                .clip(RoundedCornerShape(20.dp))
                .background(MaterialTheme.colorScheme.surfaceContainerHigh)
                .padding(horizontal = 16.dp, vertical = 14.dp),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(12.dp)
        ) {
            Icon(painterResource(R.drawable.ic_ui_lock), null, tint = MaterialTheme.colorScheme.primary)
            Text(
                stringResource(R.string.setup_privacy_promise),
                style = MaterialTheme.typography.bodyMedium,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                modifier = Modifier.weight(1f)
            )
            Text(
                stringResource(R.string.privacy_policy),
                style = MaterialTheme.typography.labelLarge,
                color = MaterialTheme.colorScheme.primary,
                modifier = Modifier.clickable { showPrivacy = true }
            )
        }
        Spacer(Modifier.height(16.dp))
        WideButton(stringResource(R.string.setup_get_started), onNext)
    }
    if (showPrivacy) {
        ModalBottomSheet(onDismissRequest = { showPrivacy = false }) {
            Box(Modifier.padding(horizontal = 16.dp).padding(bottom = 24.dp)) {
                PrivacyPolicyPage(onForward = { showPrivacy = false })
            }
        }
    }
}

@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
private fun WelcomeHero() {
    val float = rememberInfiniteTransition(label = "hero")
    val bob by float.animateFloat(-4f, 4f, infiniteRepeatable(tween(2400), RepeatMode.Reverse), label = "bob")
    Box(Modifier.size(240.dp)) {
        Box(Modifier.fillMaxSize().clip(CircleShape).background(MaterialTheme.colorScheme.primaryContainer.copy(alpha = 0.55f)))
        Box(
            Modifier
                .padding(36.dp)
                .fillMaxSize()
                .clip(MaterialShapes.Cookie9Sided.toShape())
                .background(MaterialTheme.colorScheme.primaryContainer)
        )
        Box(
            Modifier
                .offset(34.dp, 52.dp)
                .graphicsLayer { translationY = bob * density }
                .rotate(-8f)
                .size(92.dp)
                .clip(RoundedCornerShape(30.dp))
                .background(MaterialTheme.colorScheme.surfaceBright),
            contentAlignment = Alignment.Center
        ) {
            Icon(painterResource(R.drawable.ic_w_earbuds_fill), null, Modifier.size(52.dp), tint = MaterialTheme.colorScheme.primary)
        }
        Box(
            Modifier
                .offset(106.dp, 92.dp)
                .graphicsLayer { translationY = -bob * density }
                .rotate(6f)
                .size(104.dp)
                .clip(CircleShape)
                .background(MaterialTheme.colorScheme.tertiaryContainer),
            contentAlignment = Alignment.Center
        ) {
            Icon(painterResource(R.drawable.ic_w_headphones_fill), null, Modifier.size(58.dp), tint = MaterialTheme.colorScheme.onTertiaryContainer)
        }
    }
}

// ---- 2 · Permissions ----

@OptIn(ExperimentalPermissionsApi::class)
@Composable
private fun PermissionsStep(onNext: () -> Unit) {
    val context = LocalContext.current
    val prefs = remember { context.getSharedPreferences("settings", Context.MODE_PRIVATE) }
    val phone = rememberMultiplePermissionsState(
        listOf("android.permission.READ_PHONE_STATE", "android.permission.ANSWER_PHONE_CALLS")
    )
    val notifications = rememberPermissionState("android.permission.POST_NOTIFICATIONS") {
        if (!phone.allPermissionsGranted) phone.launchMultiplePermissionRequest()
    }
    val bluetooth = rememberMultiplePermissionsState(
        listOf(
            "android.permission.BLUETOOTH_CONNECT",
            "android.permission.BLUETOOTH_SCAN",
            "android.permission.BLUETOOTH_ADVERTISE"
        )
    ) { result ->
        // walk through the rest right away, one system dialog after the other
        if (result.values.all { it } && !notifications.status.isGranted) notifications.launchPermissionRequest()
    }
    val xposed = XposedState.isAvailable && XposedState.bluetoothScopeEnabled
    val rootless = remember { isSupported(prefs) }

    StepPage(
        title = stringResource(R.string.setup_permissions_title),
        subtitle = stringResource(R.string.setup_permissions_subtitle),
        content = {
            Spacer(Modifier.height(16.dp))
            SettingsGroup(
                rows = arrayOf(
                    { m -> PermissionRow(m, R.drawable.ic_ui_bluetooth_connected, stringResource(R.string.setup_perm_nearby), stringResource(R.string.setup_perm_nearby_desc), bluetooth.allPermissionsGranted, required = true) { bluetooth.launchMultiplePermissionRequest() } },
                    { m -> PermissionRow(m, R.drawable.ic_ui_notifications, stringResource(R.string.setup_perm_notifications), stringResource(R.string.setup_perm_notifications_desc), notifications.status.isGranted) { notifications.launchPermissionRequest() } },
                    { m -> PermissionRow(m, R.drawable.ic_ui_call, stringResource(R.string.setup_perm_phone), stringResource(R.string.setup_perm_phone_desc), phone.allPermissionsGranted) { phone.launchMultiplePermissionRequest() } },
                )
            )
            SectionHeader(stringResource(R.string.setup_root_features))
            val (rootTitle, rootDesc) = when {
                xposed -> stringResource(R.string.setup_root_active) to stringResource(R.string.setup_root_active_desc)
                rootless -> stringResource(R.string.setup_root_rootless) to stringResource(R.string.setup_root_rootless_desc)
                else -> stringResource(R.string.setup_root_needed) to stringResource(R.string.setup_root_needed_desc)
            }
            SettingsGroup(
                rows = arrayOf({ m ->
                    SettingsRow(
                        m, rootTitle, rootDesc,
                        icon = if (xposed || rootless) R.drawable.ic_ui_extension else R.drawable.ic_ui_warning,
                        tone = LeadTone.Tertiary,
                        trailing = if (xposed || rootless) {
                            { Icon(painterResource(R.drawable.ic_ui_check), null, tint = MaterialTheme.colorScheme.primary) }
                        } else null
                    )
                })
            )
            Text(
                stringResource(R.string.setup_root_hint),
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                modifier = Modifier.padding(start = 8.dp, end = 8.dp, top = 10.dp)
            )
        },
        bottom = {
            val allAsked = bluetooth.allPermissionsGranted && notifications.status.isGranted
            WideButton(
                text = stringResource(if (bluetooth.allPermissionsGranted) R.string.setup_continue else R.string.setup_allow),
                onClick = {
                    when {
                        !bluetooth.allPermissionsGranted -> bluetooth.launchMultiplePermissionRequest()
                        !allAsked -> notifications.launchPermissionRequest()
                        else -> {
                            // without root and on an untested phone, try anyway rather than block
                            if (!xposed && !rootless) bypassDeviceCheck(prefs)
                            onNext()
                        }
                    }
                }
            )
            if (bluetooth.allPermissionsGranted && !allAsked) {
                TextButton(onClick = {
                    if (!xposed && !rootless) bypassDeviceCheck(prefs)
                    onNext()
                }, Modifier.fillMaxWidth()) { Text(stringResource(R.string.setup_not_now)) }
            }
        }
    )
}

@Composable
private fun PermissionRow(
    modifier: Modifier,
    @DrawableRes icon: Int,
    title: String,
    subtitle: String,
    granted: Boolean,
    required: Boolean = false,
    request: () -> Unit,
) {
    SettingsRow(
        modifier, title, subtitle, icon = icon,
        tone = if (required) LeadTone.Primary else LeadTone.Secondary,
        onClick = if (granted) null else request,
        trailing = {
            if (granted) Icon(painterResource(R.drawable.ic_ui_check), null, tint = MaterialTheme.colorScheme.primary)
            else if (!required) Text(
                stringResource(R.string.setup_optional),
                style = MaterialTheme.typography.labelMedium,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                modifier = Modifier
                    .clip(RoundedCornerShape(10.dp))
                    .background(MaterialTheme.colorScheme.surfaceContainerHighest)
                    .padding(horizontal = 10.dp, vertical = 4.dp)
            )
        }
    )
}

// ---- 3 · Find AirPods ----

private data class Candidate(val mac: String, val name: String, val connected: Boolean, val isMax: Boolean)

@SuppressLint("MissingPermission")
private fun candidates(context: Context): List<Candidate> {
    val adapter = context.getSystemService(BluetoothManager::class.java)?.adapter ?: return emptyList()
    return runCatching { adapter.bondedDevices.orEmpty() }.getOrDefault(emptySet())
        .filter { d -> d.uuids?.contains(AACP_UUID) == true || looksLikeAirPods(d) }
        .map { d ->
            val name = cleanDeviceName(runCatching { d.name }.getOrNull() ?: "AirPods")
            Candidate(d.address, name, isConnected(d), name.contains("Max", ignoreCase = true))
        }
        .sortedByDescending { it.connected }
}

private fun isConnected(device: BluetoothDevice) =
    runCatching { device.javaClass.getMethod("isConnected").invoke(device) as Boolean }.getOrDefault(false)

@Composable
private fun FindStep(onChosen: (String) -> Unit, onSkip: () -> Unit) {
    val context = LocalContext.current
    var list by remember { mutableStateOf(candidates(context)) }
    var connecting by remember { mutableStateOf<String?>(null) }

    LaunchedEffect(Unit) {
        // the service does the detection; start it now rather than after setup
        runCatching { context.startForegroundService(Intent(context, AirPodsService::class.java)) }
        while (true) {
            list = candidates(context)
            connecting?.let { mac ->
                val service = ServiceManager.getService()
                if (service?.device?.address == mac && BluetoothConnectionManager.aacpSocket?.isConnected == true) {
                    onChosen(mac)
                    return@LaunchedEffect
                }
            }
            delay(1000)
        }
    }

    fun choose(c: Candidate) {
        connecting = c.mac
        context.getSharedPreferences("settings", Context.MODE_PRIVATE).edit()
            .putString("mac_address", c.mac).putString("name", c.name).apply()
        ServiceManager.getService()?.connectDevice(c.mac)
    }

    val nearby = list.firstOrNull { it.connected }
    StepPage(
        title = stringResource(R.string.setup_find_title),
        subtitle = stringResource(if (nearby != null) R.string.setup_find_found else R.string.setup_find_searching),
        content = {
            if (nearby == null) {
                SearchingPulse()
            } else {
                Spacer(Modifier.height(12.dp))
                FoundCard(nearby, connecting == nearby.mac) { choose(nearby) }
            }
            val others = list.filter { it != nearby }
            SectionHeader(stringResource(R.string.setup_paired_with_phone))
            SettingsGroup(
                rows = (others.map { c ->
                    @Composable { m: Modifier ->
                        SettingsRow(
                            m, c.name,
                            stringResource(if (c.connected) R.string.setup_connected_now else R.string.setup_not_nearby),
                            icon = if (c.isMax) R.drawable.ic_w_headphones else R.drawable.ic_w_earbuds,
                            tone = LeadTone.Secondary,
                            onClick = { choose(c) },
                            trailing = if (connecting == c.mac) {
                                { LoadingIndicatorSmall() }
                            } else null
                        )
                    }
                } + listOf<(@Composable (Modifier) -> Unit)?>({ m ->
                    SettingsRow(
                        m, stringResource(R.string.setup_pair_new), stringResource(R.string.setup_pair_new_desc),
                        icon = R.drawable.ic_w_add, tone = LeadTone.Secondary,
                        onClick = { context.startActivity(Intent(Settings.ACTION_BLUETOOTH_SETTINGS).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)) }
                    )
                })).toTypedArray()
            )
        },
        bottom = {
            Text(
                stringResource(R.string.setup_find_hint),
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                textAlign = TextAlign.Center,
                modifier = Modifier.fillMaxWidth().padding(horizontal = 16.dp)
            )
            TextButton(onClick = onSkip, modifier = Modifier.fillMaxWidth()) { Text(stringResource(R.string.setup_skip)) }
        }
    )
}

@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
private fun LoadingIndicatorSmall() = LoadingIndicator(Modifier.size(28.dp))

@Composable
private fun SearchingPulse() {
    val t = rememberInfiniteTransition(label = "pulse")
    val a by t.animateFloat(0f, 1f, infiniteRepeatable(tween(2400)), label = "a")
    val b = (a + 0.5f) % 1f
    Box(Modifier.fillMaxWidth().height(300.dp), contentAlignment = Alignment.Center) {
        listOf(a, b).forEach { p ->
            Box(
                Modifier
                    .size(260.dp)
                    .scale(0.55f + 0.45f * p)
                    .graphicsLayer { alpha = 0.7f * (1f - p) }
                    .clip(CircleShape)
                    .background(MaterialTheme.colorScheme.primaryContainer)
            )
        }
        Box(
            Modifier
                .size(120.dp)
                .clip(RoundedCornerShape(42.dp))
                .background(MaterialTheme.colorScheme.primaryContainer),
            contentAlignment = Alignment.Center
        ) {
            Icon(painterResource(R.drawable.ic_w_earbuds_fill), null, Modifier.size(60.dp), tint = MaterialTheme.colorScheme.onPrimaryContainer)
        }
    }
}

@Composable
private fun FoundCard(c: Candidate, connecting: Boolean, onUse: () -> Unit) {
    Column(
        Modifier
            .fillMaxWidth()
            .clip(RoundedCornerShape(28.dp))
            .background(MaterialTheme.colorScheme.primaryContainer)
            .padding(16.dp),
        verticalArrangement = Arrangement.spacedBy(16.dp)
    ) {
        Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(14.dp)) {
            Box(
                Modifier.size(64.dp).clip(RoundedCornerShape(22.dp)).background(MaterialTheme.colorScheme.surfaceBright),
                contentAlignment = Alignment.Center
            ) {
                Icon(
                    painterResource(if (c.isMax) R.drawable.ic_w_headphones_fill else R.drawable.ic_w_earbuds_fill), null,
                    Modifier.size(36.dp), tint = MaterialTheme.colorScheme.primary
                )
            }
            Column(Modifier.weight(1f)) {
                Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(6.dp)) {
                    Box(Modifier.size(8.dp).clip(CircleShape).background(MaterialTheme.colorScheme.primary))
                    Text(stringResource(R.string.setup_connected_now), style = MaterialTheme.typography.labelLarge, color = MaterialTheme.colorScheme.onPrimaryContainer)
                }
                Text(c.name, style = MaterialTheme.typography.titleLarge, color = MaterialTheme.colorScheme.onPrimaryContainer)
            }
        }
        Button(onClick = onUse, enabled = !connecting, modifier = Modifier.fillMaxWidth().height(56.dp)) {
            if (connecting) LoadingIndicatorSmall()
            else Text(stringResource(R.string.setup_use_these), style = MaterialTheme.typography.titleMedium)
        }
    }
}

// ---- 4 · Ready ----

@Composable
private fun DoneStep(mac: String?, onFinish: () -> Unit) {
    val context = LocalContext.current
    val prefs = remember { context.getSharedPreferences("settings", Context.MODE_PRIVATE) }
    var tick by remember { mutableIntStateOf(0) }
    LaunchedEffect(Unit) { while (true) { delay(1000); tick++ } }
    val service = remember(tick) { ServiceManager.getService() }
    val model = service?.airpodsInstance?.model
    val isMax = model is AirPodsMax
    val name = prefs.getString("name", null) ?: model?.displayName ?: "AirPods"
    val batteries = remember(tick) {
        service?.getBattery().orEmpty().filter { it.status != BatteryStatus.DISCONNECTED && it.level > 0 }
    }
    var shadeCard by remember { mutableStateOf(prefs.getBoolean("shade_card", true)) }

    StepPage(
        title = stringResource(R.string.setup_done_title),
        subtitle = stringResource(R.string.setup_done_subtitle, name),
        content = {
            Box(Modifier.fillMaxWidth().padding(vertical = 16.dp), contentAlignment = Alignment.Center) {
                Box(Modifier.size(200.dp), contentAlignment = Alignment.Center) {
                    Box(Modifier.fillMaxSize().clip(MaterialShapes.Cookie9Sided.toShape()).background(MaterialTheme.colorScheme.primaryContainer))
                    if (model != null && !isMax) Image(painterResource(model.budsRes), null, Modifier.size(150.dp))
                    else Icon(painterResource(if (isMax || mac == null) R.drawable.ic_w_headphones_fill else R.drawable.ic_w_earbuds_fill), null, Modifier.size(96.dp), tint = MaterialTheme.colorScheme.onPrimaryContainer)
                    Box(
                        Modifier
                            .align(Alignment.BottomEnd)
                            .offset((-8).dp, (-14).dp)
                            .size(48.dp)
                            .clip(CircleShape)
                            .background(MaterialTheme.colorScheme.surfaceContainer)
                            .padding(4.dp)
                            .clip(CircleShape)
                            .background(MaterialTheme.colorScheme.primary),
                        contentAlignment = Alignment.Center
                    ) { Icon(painterResource(R.drawable.ic_ui_check), null, tint = MaterialTheme.colorScheme.onPrimary) }
                }
            }
            if (batteries.isNotEmpty()) {
                Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(8.dp, Alignment.CenterHorizontally)) {
                    batteries.forEach { b ->
                        val label = when {
                            isMax -> ""
                            b.component == BatteryComponent.LEFT -> " L"
                            b.component == BatteryComponent.RIGHT -> " R"
                            else -> " " + stringResource(R.string.case_alt)
                        }
                        Text(
                            "${b.level}%$label",
                            style = MaterialTheme.typography.labelLarge,
                            fontSize = 14.sp,
                            modifier = Modifier
                                .clip(RoundedCornerShape(16.dp))
                                .background(MaterialTheme.colorScheme.surfaceContainerHigh)
                                .padding(horizontal = 14.dp, vertical = 8.dp)
                        )
                    }
                }
            }
            SectionHeader(stringResource(R.string.setup_built_in))
            SettingsGroup(
                rows = arrayOf(
                    if (XposedState.systemUiScopeEnabled) { m ->
                        SwitchRow(
                            m, stringResource(R.string.ui_shade_card), stringResource(R.string.setup_shade_card_desc),
                            checked = shadeCard,
                            onCheckedChange = { shadeCard = it; prefs.edit().putBoolean("shade_card", it).apply() },
                            icon = R.drawable.ic_ui_notifications, tone = LeadTone.Tertiary
                        )
                    } else null,
                    { m ->
                        SettingsRow(
                            m, stringResource(R.string.setup_add_widget), stringResource(R.string.setup_add_widget_desc),
                            icon = R.drawable.ic_ui_widgets, tone = LeadTone.Tertiary,
                            onClick = {
                                val manager = context.getSystemService(AppWidgetManager::class.java)
                                if (manager.isRequestPinAppWidgetSupported) {
                                    manager.requestPinAppWidget(ComponentName(context, BatteryWidget::class.java), null, null)
                                }
                            }
                        )
                    },
                    { m ->
                        SettingsRow(
                            m, stringResource(R.string.setup_qs_tile), stringResource(R.string.setup_qs_tile_desc),
                            icon = R.drawable.ic_ui_tile_small, tone = LeadTone.Tertiary,
                            onClick = {
                                context.getSystemService(StatusBarManager::class.java)?.requestAddTileService(
                                    ComponentName(context, AirPodsQSService::class.java),
                                    context.getString(R.string.ui_qs_title),
                                    Icon.createWithResource(context, R.drawable.ic_w_earbuds_fill),
                                    context.mainExecutor
                                ) { }
                            }
                        )
                    },
                )
            )
        },
        bottom = { WideButton(stringResource(R.string.setup_open_app), onFinish) }
    )
}

