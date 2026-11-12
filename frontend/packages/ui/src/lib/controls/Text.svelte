<script lang="ts">
  export let id: string;
  export let value: string;
  export let placeholder: string | undefined = undefined;
  export let disabled = false;
  export let onchange: (value: string) => void;
</script>

<input
  {id}
  type="text"
  {value}
  {placeholder}
  {disabled}
  oninput={(event) => onchange(event.currentTarget.value)}
/>
