<script lang="ts">
  export let id: string;
  export let value: string;
  export let disabled = false;
  export let onchange: (value: string) => void;
</script>

<div class="color-field">
  <input
    {id}
    type="color"
    {value}
    {disabled}
    oninput={(event) => onchange(event.currentTarget.value)}
  />
  <code>{value}</code>
</div>
