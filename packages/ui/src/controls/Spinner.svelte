<script lang="ts"></script>

<svg class="st-spinner" width="14" height="14" viewBox="0 0 16 16" fill="none" role="progressbar" aria-label="Loading">
  <circle cx="8" cy="8" r="6" stroke="currentColor" stroke-opacity=".25" stroke-width="2" />
  <path d="M8 2a6 6 0 0 1 6 6" stroke="currentColor" stroke-width="2" stroke-linecap="round" />
</svg>

<style>
  .st-spinner {
    animation: spin 0.8s linear infinite;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .st-spinner {
      animation-duration: 2.4s;
    }
  }
</style>
