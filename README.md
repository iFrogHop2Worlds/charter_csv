# Charter csv
Is a cross-platform program to navigate and visualize the data in your csv files. The main goal of this project
is to provide a free tool that is highly performant both at the software level and user workflow level. This program
was designed around working with csv files but can be used with just about any data if it exists in a database 
we support.

# Features
- Out of the box support for large files. No need to configure any special setup or pay for a premium plan 
just to work with millions or billions of rows of data, we give you that for free. 
- Cross file queries. Allows powerful queries with no db or sql. 
- Simple intuitive query builder. No need to learn any query languages.   
- Simple csv file editor.
- Export graphs and charts as images.
- Sessions allow you to save and reconstruct your session state, for a consistent, reliable experience and greater depth of analysis.
- Database support. We include a sql-lite db and currently support sql queries. You can also hook into you own database using your
connection string. (Posgresql and Mongodb will be next to be supported)

# CSVQB
I am playing with the idea of using common math and logical operators to construct pipelines which can query and perform
transformations on your data. Were calling this csvqb or 'csv query builder'.

csvqb is in early development and far from complete. csvqb uses a hybrid infix and reverse polish notation approach
to construct operation pipelines. Operations are performed on a 2dVector representation of the csv files. The benefit 
of using CSVQB is you can quickly and easily extract meaningful data in seconds without thinking or worrying about
any complex sql and no need to port your files it into a database. Just conventiently work with data on the fly as you need.

**todo - add csvqb examples + quickstart guide**

# Future road map
Future versions will have:
- Ai assistant to construct pipelines.
- Device sync.
- Simulations.
- Improved file editor.
- Cross file pipelines.
- Additional graphing features.
- Teams/Organization support.
- Refactored codebase and continuous updates including performance optimizations.

please feel free to fork, make pr's and raise issues.

**todo - add contribution guide** 
