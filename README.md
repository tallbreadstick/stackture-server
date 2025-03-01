# Stackture - Structure Your Stack (Server)

An application that guides learners through a precise problem-focused strategy.

**Project Goal/Purpose:** 

Stackture targets SDG 4, "Ensure inclusive and equitable quality education and promote lifelong learning opportunities for all.”

Humans are the most important resources worth cultivating sustainably. Beyond ecosystems, economies, and industries, developing human minds and skills for future generations will be the most beneficial investment we can make in the present. To assist the youth of this generation undergoing education, Stackture aims to be a study tool that offers precise guidance in solving problems, overcoming learning gaps, and reaching goals.

**Tech Stack**<br>
[![My Skills](https://go-skill-icons.vercel.app/api/icons?i=rust,postgresql)](https://skillicons.dev)

This is a part of **Stackture**.<br>
For more info click on the main repo link below.<br>
**Main repo link:**<br>
https://github.com/DymNomZ/Stackture

**Property of 5stack as part of the University of the Philippines' KOMSAI WEEK 2025 HACKATHON**


# API Documentation

### Register

Endpoint

    POST http://stackture.eloquenceprojects.org/auth/register

Headers

    Content-Type: application/json

Body

    {
        "username": "{username}",
        "email": "{email}",
        "password": "{password}"
    }

Success (200 OK)

    {
        "token": "{jwt}"
    }

Error

    {
        "error": "{reason}"
    }

### Login

Endpoint

    POST http://stackture.eloquenceprojects.org/auth/login

Headers

    Content-Type: application/json

Body

    {
        "username": "{username}",
        "password": "{password}"
    }

Success (200 OK)

    {
        "token": "{jwt}"
    }

Error

    {
        "error": "{reason}"
    }

### Create Workspace

Endpoint

    POST http://stackture.eloquenceprojects.org/api/workspace/create

Headers

    Authorization: Bearer {jwt}
    Content-Type: application/json

Body

    {
        "title": "{title}",
        "description": "{some_description_about_the_workspace}"
    }

Success (201 CREATED)

    {
        "workspace_id": {id}
    }

Error

    {
        "error": "{reason}"
    }

### Fetch Workspaces

Endpoint

    GET http://stackture.eloquenceprojects.org/api/workspace/fetch

Headers

    Authorization: Bearer {jwt}

Success (200 OK)

    // EXAMPLE ONLY, returns a json list of all workspaces

    [
        {
            "id": 42,
            "title": "Physics Learning",
            "description": "Tracking my progress in physics",
            "root_id": 1
        },
        {
            "id": 43,
            "title": "Math Study",
            "description": "Algebra and calculus",
            "root_id": 10
        }
    ]

Error

    {
        "error": "{reason}"
    }

### Get Workspace

Endpoint

    GET http://stackture.eloquenceprojects.org/api/workspace/get/{id}

Headers

    Authorization: Bearer {jwt}

Success (200 OK)

    // EXAMPLE ONLY, returns a json list of all nodes in the workspace representing the state of the tree

    [
        {
            "id": 1,
            "name": "Root Problem",
            "summary": "The main problem to solve.",
            "optional": false,
            "resolved": false,
            "icon": "📌",
            "branches": [2, 3],
            "parents": []
        },
        {
            "id": 2,
            "name": "Subproblem A",
            "summary": "A required step.",
            "optional": false,
            "resolved": false,
            "icon": "📎",
            "branches": [],
            "parents": [1]
        },
        {
            "id": 3,
            "name": "Subproblem B",
            "summary": "An alternative path.",
            "optional": true,
            "resolved": false,
            "icon": "📎",
            "branches": [],
            "parents": [1]
        }
    ]

Error

    {
        "error": "{reason}"
    }

### Delete Workspace

Endpoint

    DELETE http://stackture.eloquenceprojects.org/api/workspace/delete/{id}

Headers

    Authorization: Bearer {jwt}

Success (204 NO CONTENT)

Error

    {
        "error": "{reason}"
    }