<script lang="ts">
  export let id: string;
  export let checked = false;
  export let disabled = false;
  export let onchange: (value: boolean) => void;
</script>

<label class="switch" class:disabled>
  <input
    {id}
    type="checkbox"
    {checked}
    {disabled}
    onchange={(event) => onchange(event.currentTarget.checked)}
  />
  <span class="track" aria-hidden="true"><span class="thumb"></span></span>
</label>
