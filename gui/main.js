const libraryContainer = document.getElementById('library');

fetch('./library')
  .then((response) => response.json())
  .then((library) => {
    libraryContainer.innerHTML = '';
    library.forEach((e) => {
      const item = document.createElement('div');
      item.innerHTML = `${e.title}`;
      libraryContainer.appendChild(item);
    })
});
