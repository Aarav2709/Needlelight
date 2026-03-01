<template>
<ModalWrapper ref="modal" header="Create a new instance">
<div class="modal-body">
<div class="image-upload">
<Avatar :src="display_icon" size="md" :rounded="true" />
<div class="image-input">
<Button @click="upload_icon()">
<UploadIcon />
Select icon
</Button>
<Button :disabled="!display_icon" @click="reset_icon">
<XIcon />
Remove icon
</Button>
</div>
</div>
<div class="input-row">
<p class="input-label">Name</p>
<StyledInput
v-model="profile_name"
autocomplete="off"
type="text"
placeholder="Enter a name for your instance..."
:maxlength="100"
wrapper-class="w-full"
/>
</div>
<div class="input-group push-right">
<Button @click="hide()">
<XIcon />
Cancel
</Button>
<Button color="primary" :disabled="!check_valid || creating" @click="create_instance()">
<PlusIcon v-if="!creating" />
{{ creating ? 'Creating...' : 'Create' }}
</Button>
</div>
</div>
</ModalWrapper>
</template>

<script setup>
import {
PlusIcon,
UploadIcon,
XIcon,
} from '@modrinth/assets'
import {
Avatar,
Button,
injectNotificationManager,
StyledInput,
} from '@modrinth/ui'
import { convertFileSrc } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-dialog'
import { computed, ref } from 'vue'

import ModalWrapper from '@/components/ui/modal/ModalWrapper.vue'
import { create } from '@/helpers/profile'

const { handleError } = injectNotificationManager()

const profile_name = ref('')
const icon = ref(null)
const display_icon = ref(null)
const creating = ref(false)

defineExpose({
show: async () => {
profile_name.value = ''
creating.value = false
icon.value = null
display_icon.value = null
modal.value.show()
},
})

const hide = () => {
modal.value.hide()
}

const modal = ref(null)

const check_valid = computed(() => {
return profile_name.value.trim().length > 0
})

const create_instance = async () => {
creating.value = true
hide()
creating.value = false

await create(
profile_name.value,
'1.0',
'hollow_knight',
null,
icon.value,
).catch(handleError)
}

const upload_icon = async () => {
const res = await open({
multiple: false,
filters: [
{
name: 'Image',
extensions: ['png', 'jpeg', 'svg', 'webp', 'gif', 'jpg'],
},
],
})

icon.value = res.path ?? res

if (!icon.value) return
display_icon.value = convertFileSrc(icon.value)
}

const reset_icon = () => {
icon.value = null
display_icon.value = null
}
</script>

<style lang="scss" scoped>
.modal-body {
display: flex;
flex-direction: column;
gap: var(--gap-md);
margin-top: var(--gap-lg);
}

.input-label {
font-size: 1rem;
font-weight: bolder;
color: var(--color-contrast);
margin-bottom: 0.5rem;
}

.image-upload {
display: flex;
gap: 1rem;
}

.image-input {
display: flex;
flex-direction: column;
gap: 0.5rem;
justify-content: center;
}
</style>
