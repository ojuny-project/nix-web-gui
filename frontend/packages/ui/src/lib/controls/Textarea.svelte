<script lang="ts">
  export let id: string;
  export let value: string;
  export let placeholder: string | undefined = undefined;
  export let rows = 6;
  export let disabled = false;
  export let onchange: (value: string) => void;
</script>

<textarea
  {id}
  {value}
  {placeholder}
  {rows}
  {disabled}
  oninput={(event) => onchange(event.currentTarget.value)}
></textarea>
