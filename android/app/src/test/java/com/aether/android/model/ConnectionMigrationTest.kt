package com.aether.android.model
import org.junit.Assert.*
import org.junit.Test
class ConnectionMigrationTest {
 @Test fun oldInstallKeepsManualProtocolAndAppSelection(){
  val settings=SettingsMigration.decode("""{"vpnProtocol":"WIREGUARD","selectedPackages":["org.example.app"],"splitTunnelMode":"ONLY_SELECTED","enableFragmentation":true}""")
  assertEquals(ConnectionMode.MANUAL,settings.connectionMode)
  assertEquals(VpnProtocol.WIREGUARD,settings.vpnProtocol)
  assertEquals(setOf("org.example.app"),settings.selectedPackages)
  assertEquals(SplitTunnelMode.ONLY_SELECTED,settings.splitTunnelMode)
  assertTrue(settings.enableFragmentation)
 }
 @Test fun cachedNetworkRetryIsBoundedAndTorIsOptIn(){
  val attempts=ConnectionProfile.autoAttempts(ConnectionProfile.MASQUE_H2)
  assertEquals(25L,attempts.first().second)
  assertEquals(listOf(ConnectionProfile.MASQUE_H2,ConnectionProfile.MASQUE_H2,ConnectionProfile.MASQUE_H3,ConnectionProfile.PSIPHON_AUTO,ConnectionProfile.PSIPHON_CDN),attempts.map{it.first})
  assertFalse(ConnectionProfile.autoAttempts(ConnectionProfile.TOR).any{it.first==ConnectionProfile.TOR})
 }
 @Test fun publicTcpTransportCannotClaimUdp(){assertFalse(ConnectionProfile.PSIPHON_AUTO.udp);assertFalse(ConnectionProfile.PSIPHON_CDN.udp);assertFalse(ConnectionProfile.TOR.udp);assertTrue(ConnectionProfile.PSIPHON_REVERSE.udp)}
 @org.junit.Test fun deepManualScanGetsItsFullBudgetWithoutChangingAuto() {
  val deep=ConnectionProfile.MASQUE_H2.manualAttempt(true)
  org.junit.Assert.assertEquals(340L,deep.second)
  org.junit.Assert.assertFalse(deep.third)
  org.junit.Assert.assertTrue(ConnectionProfile.autoAttempts(null).all { it.second<=180L })
 }
}
