echo Wrap Images
./target/debug/presentation -v -f -o tests/presentations/wrap_images.rmd tests/head_page.rmd tests/notebooks/wrap_images.ipynb

echo Folders and subfolders
./target/debug/presentation -v -f -o tests/presentations/multiple_books.rmd tests/head_page.rmd tests/notebooks/multiple_books

echo classes
./target/debug/presentation -v -f -o tests/presentations/class.rmd tests/head_page.rmd tests/notebooks/class.ipynb



echo All pages
./target/debug/presentation -v -f -o tests/presentations/all.rmd tests/head_page.rmd tests/notebooks/wrap_images.ipynb tests/notebooks/multiple_books tests/notebooks/class.ipynb