<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref } from 'vue';
import { mountStory } from './diagrams/player';
import { stories, type StoryName } from './diagrams/stories';

const props = defineProps<{ name: StoryName }>();
const host = ref<HTMLElement>();
const markup = stories[props.name].markup();
let stop: (() => void) | undefined;

onMounted(() => {
  const root = host.value?.firstElementChild;
  if (root instanceof HTMLElement) stop = mountStory(root, stories[props.name]);
});
onBeforeUnmount(() => stop?.());
</script>

<template>
  <div ref="host" class="rc-story-host" v-html="markup" />
</template>
