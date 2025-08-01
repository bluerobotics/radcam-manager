<template>
  <v-dialog                                           
    :model-value="isLoading"  
    persistent                                        
    width="300px"  
  >                                                   
    <div                                              
      class="loading-card flex flex-col items-center justify-center pt-[10px] rounded-lg" 
    >                                                 
      <img src="../../public/assets/logo.svg" class="w-[120px] h-[120px] mb-1" alt="logo" />              
      <span class="w-[200px] text-[#e9f5e6] text-center text-lg mb-6">Applying settings…</span>                     
      <v-progress-linear indeterminate color="#FFFFFF44" class="w-full" />
    </div>                                            
  </v-dialog>   
</template>
<script setup lang="ts">
  const { isLoading } = defineProps<{
    isLoading: boolean
  }>()
</script>
<style scoped>
  .loading-card {
    background-color: #10101065;
    backdrop-filter: blur(5px);
    border: 1px solid rgba(255, 255, 255, 0.08);
    box-shadow: 0px 4px 4px 0px #00000033, 0px 8px 12px 6px #00000026;
  }
</style>