<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { WalletTab, NetworkTab, UTXOTab } from './components'
import { useNeptuneWallet } from '@/composables/useNeptuneWallet'

const neptuneWallet = useNeptuneWallet()

const activeTab = ref('WALLET')
const network = ref('xnt-mainnet')

const tabItems = [
    { key: 'WALLET', label: 'WALLET' },
    { key: 'UTXOs', label: 'UTXOs' },
    { key: 'NETWORK', label: 'NETWORK' },
]

onMounted(async () => {
    await neptuneWallet.getNetworkInfo()
})
</script>

<template>
    <div class="home-container">
        <TabsCommon v-model="activeTab" :items="tabItems" size="large" class="main-tabs">
            <!-- WALLET TAB -->
            <TabPaneCommon tab-key="WALLET">
                <WalletTab :network="network" />
            </TabPaneCommon>

            <!-- UTXO TAB -->
            <TabPaneCommon tab-key="UTXOs">
                <UTXOTab />
            </TabPaneCommon>

            <!-- NETWORK TAB -->
            <TabPaneCommon tab-key="NETWORK">
                <NetworkTab />
            </TabPaneCommon>
        </TabsCommon>
    </div>
</template>

<style lang="scss" scoped>
.home-container {
    height: 100%;
    display: flex;
    flex-direction: column;
    padding: var(--spacing-lg);

    @include screen(tablet) {
        padding: var(--spacing-xl);
    }

    @include screen(desktop) {
        padding: var(--spacing-xl);
    }
}

.main-tabs {
    height: 100%;
}
</style>
